use std::collections::{HashMap, HashSet};
use std::vec;
use std::{net::IpAddr, sync::Arc, time::Duration};

use crate::membership_manager::client_sessions;
use crate::proto::packet_wrapper::PacketType;
use crate::proto::retransmission_packet::RetransmissionType;
use crate::proto::RetransmissionPacket;
use crate::proto::{PacketWrapper, XorPacket};
use crate::{membership_manager::client_sessions::ClientSessions, MATRIX_BUFFER};
use crate::{CONFIG, MANAGER};
use iris_shared::multicast_retransmission::SendAction;
use iris_shared::{multicast_retransmission, next_multiple_of_64, padding};

use crossbeam_channel::{tick, Sender};
use log::{debug, error, info};
use openssl::ecdsa::EcdsaSig;
use openssl::sha::sha256;
use prost::Message;
use reed_solomon_16::ReedSolomonEncoder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Multicast,
    Unicast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseConfig {
    pub phase: Phase,
    pub enabled: bool,
}

impl Phase {
    fn next(self) -> Self {
        match self {
            Phase::Multicast => Phase::Unicast,
            Phase::Unicast => Phase::Multicast,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PacketSentInfo {
    phase_index: usize,
    round: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RetransmissionKey {
    matrix_id: usize,
    action: SendAction,
}

pub struct RetransmissionManager {
    packet_states: HashMap<RetransmissionKey, PacketSentInfo>,
    phases: Vec<PhaseConfig>,
    phase_index: usize,
    round: usize,
}

impl RetransmissionManager {
    pub fn new(phases: Vec<PhaseConfig>) -> Self {
        assert!(!phases.is_empty());
        Self {
            packet_states: HashMap::new(),
            phases,
            phase_index: 0,
            round: 0,
        }
    }

    fn advance_phase(&mut self) {
        let num_phases = self.phases.len();
        for _ in 0..num_phases {
            self.phase_index = (self.phase_index + 1) % num_phases;
            if self.phase_index == 0 {
                self.round += 1;
            }
            if self.phases[self.phase_index].enabled {
                return;
            }
        }
    }

    fn can_retransmit(
        &self,
        last_sent: &PacketSentInfo,
        current_phase_index: usize,
        current_round: usize,
    ) -> bool {
        !(last_sent.phase_index == current_phase_index && last_sent.round == current_round)
    }

    pub fn handle_retransmissions(
        &mut self,
        client_sessions: Arc<ClientSessions>,
        multicast_tx: Sender<Vec<u8>>,
        unicast_tx: Sender<(Vec<u8>, IpAddr)>,
    ) {
        let config = CONFIG.get().unwrap();
        let matrix_buffer = MATRIX_BUFFER.get().unwrap();
        let manager = MANAGER.get().unwrap();

        let ec_key = {
            let binding = manager.read().unwrap();
            binding.get_ec_key().clone()
        };

        let interval_ticker = tick(Duration::from_millis(config.retransmission.interval as u64));

        for _ in interval_ticker {
            let phase_config = self.phases[self.phase_index];
            if !phase_config.enabled {
                self.advance_phase();
                continue;
            }

            let current_phase = phase_config.phase;

            match current_phase {
                Phase::Multicast => {
                    let nc_matrix = client_sessions.generate_nc_matrix();
                    let multicast_bandwidth_manager = &client_sessions.mutlicast_bandwidth_manager;

                    for (matrix_id, (mut matrix, _clients_vector)) in nc_matrix {
                        if matrix.is_empty() {
                            println!("Matrix {} is empty, skipping.", matrix_id);
                            continue;
                        }

                        let already_sent: HashSet<SendAction> = self
                            .packet_states
                            .iter()
                            .filter_map(|(key, info)| {
                                if key.matrix_id == matrix_id as usize
                                    && !self.can_retransmit(info, self.phase_index, self.round)
                                {
                                    Some(key.action.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let sent_packets =
                            multicast_retransmission::compute(&mut matrix, &already_sent);

                        let buffer_matrix = match matrix_buffer.get_matrix(matrix_id as usize) {
                            Some(m) => m,
                            None => {
                                error!("Matrix {} not found in buffer, skipping.", matrix_id);
                                continue;
                            }
                        };

                        for pkt in sent_packets {
                            self.packet_states.insert(
                                RetransmissionKey {
                                    matrix_id: matrix_id.try_into().unwrap(),
                                    action: pkt.clone(),
                                },
                                PacketSentInfo {
                                    phase_index: self.phase_index,
                                    round: self.round,
                                },
                            );

                            match pkt {
                                SendAction::Packet(idx) => {
                                    let row = idx / buffer_matrix.cols() as usize;
                                    let col = idx % buffer_matrix.cols() as usize;

                                    if let Some(packet) = buffer_matrix.get(row, col) {
                                        let wrapper = PacketWrapper {
                                            packet_type: Some(PacketType::RetransmissionPacket(
                                                RetransmissionPacket {
                                                    retransmission_type: Some(
                                                        RetransmissionType::TransmissionPacket(
                                                            packet.clone(),
                                                        ),
                                                    ),
                                                },
                                            )),
                                        };

                                        let wrapper_vec = wrapper.encode_to_vec();
                                        let can_send =
                                            multicast_bandwidth_manager.try_send(wrapper_vec.len());

                                        if !can_send {
                                            debug!(
                                                "Cannot send packet {}, bandwidth limit reached.",
                                                idx
                                            );
                                            continue;
                                        }

                                        multicast_tx
                                            .send(wrapper.encode_to_vec())
                                            .expect("Failed to send multicast packet");
                                    } else {
                                        error!(
                                            "Packet {} not found in matrix {}, skipping.",
                                            idx, matrix_id
                                        );
                                    }
                                }
                                SendAction::Xor(i, j) => {
                                    let mut shard_i = match buffer_matrix.get(
                                        i / buffer_matrix.cols() as usize,
                                        i % buffer_matrix.cols() as usize,
                                    ) {
                                        Some(p) => p.encode_to_vec(),
                                        None => {
                                            error!(
                                                "Packet {} not found in matrix {}, skipping.",
                                                i, matrix_id
                                            );
                                            continue;
                                        }
                                    };
                                    let mut shard_j = match buffer_matrix.get(
                                        j / buffer_matrix.cols() as usize,
                                        j % buffer_matrix.cols() as usize,
                                    ) {
                                        Some(p) => p.encode_to_vec(),
                                        None => {
                                            error!(
                                                "Packet {} not found in matrix {}, skipping.",
                                                j, matrix_id
                                            );
                                            continue;
                                        }
                                    };

                                    let max_len = shard_i.len().max(shard_j.len());
                                    let shard_size = next_multiple_of_64(max_len as u32);

                                    padding::pad(&mut shard_i, shard_size as usize);
                                    padding::pad(&mut shard_j, shard_size as usize);

                                    let mut encoder =
                                        match ReedSolomonEncoder::new(2, 1, shard_size as usize) {
                                            Ok(enc) => enc,
                                            Err(e) => {
                                                error!(
                                                    "Failed to create Reed-Solomon encoder: {}",
                                                    e
                                                );
                                                continue;
                                            }
                                        };

                                    if let Err(e) = encoder.add_original_shard(shard_i) {
                                        error!("Failed to add shard i: {}", e);
                                        continue;
                                    }
                                    if let Err(e) = encoder.add_original_shard(shard_j) {
                                        error!("Failed to add shard j: {}", e);
                                        continue;
                                    }

                                    let recovery_shard = match encoder.encode() {
                                        Ok(r) => r.recovery_iter().next().unwrap().to_vec(),
                                        Err(e) => {
                                            error!(
                                                "Failed to encode packets with Reed-Solomon: {}",
                                                e
                                            );
                                            continue;
                                        }
                                    };

                                    let mut xor_packet = XorPacket {
                                        matrix: matrix_id as u32,
                                        indexes: vec![i as u8, j as u8],
                                        payload: recovery_shard,
                                        packet_signature: vec![],
                                    };

                                    let message_hash = sha256(&xor_packet.encode_to_vec());
                                    let signature = match EcdsaSig::sign(&message_hash, &ec_key) {
                                        Ok(sig) => sig.to_der().unwrap_or_default(),
                                        Err(e) => {
                                            error!("Failed to sign XOR packet: {}", e);
                                            continue;
                                        }
                                    };
                                    xor_packet.packet_signature = signature;

                                    let wrapper = PacketWrapper {
                                        packet_type: Some(PacketType::RetransmissionPacket(
                                            RetransmissionPacket {
                                                retransmission_type: Some(
                                                    RetransmissionType::XorPacket(xor_packet),
                                                ),
                                            },
                                        )),
                                    };

                                    let wrapper_vec = wrapper.encode_to_vec();
                                    let can_send =
                                        multicast_bandwidth_manager.try_send(wrapper_vec.len());

                                    if !can_send {
                                        debug!(
                                            "Cannot send XOR packet {}, bandwidth limit reached.",
                                            format!("{},{}", i, j)
                                        );
                                        continue;
                                    }

                                    if let Err(e) = multicast_tx.send(wrapper.encode_to_vec()) {
                                        error!("Failed to send multicast channel: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }

                Phase::Unicast => {
                    let nc_matrix = client_sessions.generate_nc_matrix();

                    for (matrix_id, (nc_matrix, clients_vector)) in nc_matrix {
                        if nc_matrix.is_empty() {
                            continue;
                        }

                        let buffer_matrix = match matrix_buffer.get_matrix(matrix_id as usize) {
                            Some(m) => m,
                            None => {
                                error!("Matrix {} not found in buffer, skipping.", matrix_id);
                                continue;
                            }
                        };

                        for (client_index, row_iterator) in nc_matrix.iter_rows().enumerate() {
                            let row_vector: Vec<u8> = row_iterator.cloned().collect();
                            let missing_indices = client_sessions::find_zero_indices(&row_vector);

                            if missing_indices.is_empty() {
                                debug!(
                                    "No missing indices for client {} in matrix {}, skipping.",
                                    client_index, matrix_id
                                );
                                continue;
                            }

                            let client_addr = match clients_vector.get(client_index) {
                                Some(addr) => addr,
                                None => {
                                    error!(
                                        "No client address for index {} in matrix {}",
                                        client_index, matrix_id
                                    );
                                    continue;
                                }
                            };

                            for packet_idx in missing_indices {
                                let action = SendAction::Packet(packet_idx as usize);
                                let key = RetransmissionKey {
                                    matrix_id: matrix_id.try_into().unwrap(),
                                    action: action.clone(),
                                };

                                let should_send = match self.packet_states.get(&key) {
                                    Some(info) => {
                                        self.can_retransmit(info, self.phase_index, self.round)
                                    }
                                    None => true,
                                };

                                if !should_send {
                                    continue;
                                }

                                self.packet_states.insert(
                                    key,
                                    PacketSentInfo {
                                        phase_index: self.phase_index,
                                        round: self.round,
                                    },
                                );

                                let row = packet_idx as usize / buffer_matrix.cols() as usize;
                                let col = packet_idx as usize % buffer_matrix.cols() as usize;

                                if let Some(packet) = buffer_matrix.get(row, col) {
                                    let wrapper = PacketWrapper {
                                        packet_type: Some(PacketType::RetransmissionPacket(
                                            RetransmissionPacket {
                                                retransmission_type: Some(
                                                    RetransmissionType::TransmissionPacket(
                                                        packet.clone(),
                                                    ),
                                                ),
                                            },
                                        )),
                                    };

                                    let wrapper_vec = wrapper.encode_to_vec();

                                    let client_entry =
                                        client_sessions.get_client_data(&client_addr.0);

                                    if client_entry.is_none() {
                                        error!(
                                            "Client data for {:?} not found, skipping.",
                                            client_addr
                                        );
                                        continue;
                                    }

                                    let can_send = client_sessions
                                        .client_bandwidth_manager
                                        .try_send(&client_addr.0, wrapper_vec.len());

                                    if !can_send {
                                        debug!(
                                            "Cannot send packet to {:?}, bandwidth limit reached.",
                                            client_addr
                                        );
                                        continue;
                                    }

                                    if let Err(e) =
                                        unicast_tx.send((wrapper.encode_to_vec(), client_addr.1))
                                    {
                                        error!(
                                            "Failed to send unicast packet to {:?}: {}",
                                            client_addr, e
                                        );
                                    }
                                } else {
                                    error!(
                                        "Packet {} not found in matrix {}, skipping.",
                                        packet_idx, matrix_id
                                    );
                                }
                            }
                        }
                    }
                }
            }

            self.advance_phase();
        }
    }
}
