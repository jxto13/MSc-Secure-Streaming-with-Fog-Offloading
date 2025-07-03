use std::collections::{HashMap, HashSet};
use std::vec;
use std::{net::IpAddr, sync::Arc, time::Duration};

use crate::proto::packet_wrapper::PacketType;
use crate::proto::retransmission_packet::RetransmissionType;
use crate::proto::{PacketWrapper, XorPacket};
use crate::{membership_manager::client_sessions::ClientSessions, MATRIX_BUFFER};
use crate::{metrics::get_metrics, proto::RetransmissionPacket};
use crate::{CONFIG, MANAGER};
use grid::Grid;
use iris_shared::multicast_retransmission::SendAction;
use iris_shared::source_coding::{find_optimal_retransmissions, Transmission};
use iris_shared::{multicast_retransmission, next_multiple_of_64, padding};

use crossbeam_channel::{select, tick, Sender};
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

impl Phase {
    fn next(self) -> Self {
        match self {
            Phase::Multicast => Phase::Unicast,
            Phase::Unicast => Phase::Multicast,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PacketKey {
    matrix_id: usize,
    packet_idx: usize,
}

// Track when a packet was last sent: in which phase and round
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PacketSentInfo {
    phase_index: usize,
    round: usize,
}

pub struct RetransmissionManager {
    packet_states: HashMap<PacketKey, PacketSentInfo>,
    phases: Vec<Phase>, // allow variable phases
    phase_index: usize,
    round: usize,
}

impl RetransmissionManager {
    pub fn new(phases: Vec<Phase>) -> Self {
        assert!(!phases.is_empty());
        Self {
            packet_states: HashMap::new(),
            phases,
            phase_index: 0,
            round: 0,
        }
    }

    fn advance_phase(&mut self) {
        self.phase_index += 1;
        if self.phase_index >= self.phases.len() {
            self.phase_index = 0;
            self.round += 1;
        }
    }

    // returns true if (current_phase_index, current_round) is at least
    // one full cycle after (last_sent_phase_index, last_sent_round)
    fn can_retransmit(
        &self,
        last_sent: &PacketSentInfo,
        current_phase_index: usize,
        current_round: usize,
    ) -> bool {
        let phases_count = self.phases.len();
        let last_pos = last_sent.round * phases_count + last_sent.phase_index;
        let current_pos = current_round * phases_count + current_phase_index;

        // allow retransmit only if we advanced at least a full cycle ahead
        current_pos > last_pos + (phases_count - 1)
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

        // Get the current EC key from the manager
        let ec_key = {
            let binding = manager.read().unwrap();
            binding.get_ec_key().clone()
        };

        // Set global interval ticker
        let interval_ticker = tick(Duration::from_millis(config.retransmission.interval as u64));

        for _ in interval_ticker {
            let current_phase = self.phases[self.phase_index];
            println!("Round {}, Phase {:?}", self.round, current_phase);

            match current_phase {
                Phase::Multicast => {
                    let nc_matrix = client_sessions.generate_nc_matrix();

                    for (matrix_id, (mut matrix, _clients_vector)) in nc_matrix {
                        if matrix.is_empty() {
                            println!("Matrix {} is empty, skipping.", matrix_id);
                            continue;
                        }

                        println!("Matrix {}:", matrix_id);

                        // Build already_sent set from packet_states for this phase
                        let already_sent: HashSet<SendAction> = self
                            .packet_states
                            .iter()
                            .filter_map(|(key, info)| {
                                if !self.can_retransmit(info, self.phase_index, self.round) {
                                    Some(SendAction::Packet(key.packet_idx))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        // Here, multicast packets
                        let sent_packets =
                            multicast_retransmission::compute(&mut matrix, &already_sent);

                        let matrix = match matrix_buffer.get_matrix(matrix_id as usize) {
                            Some(m) => m,
                            None => {
                                error!("Matrix {} not found in buffer, skipping.", matrix_id);
                                continue;
                            }
                        };

                        for pkt in sent_packets {
                            let packet_idx = match pkt {
                                SendAction::Packet(idx) => idx,
                                SendAction::Xor(i, _) => i,
                            };

                            let key = PacketKey {
                                matrix_id: matrix_id as usize,
                                packet_idx,
                            };

                            self.packet_states.insert(
                                key,
                                PacketSentInfo {
                                    phase_index: self.phase_index,
                                    round: self.round,
                                },
                            );

                            match pkt {
                                SendAction::Packet(idx) => {
                                    debug!("Multicast packet {}", idx);
                                    // Build the RetransmissionPacket with packet info
                                    let row = idx / matrix.cols() as usize;
                                    let col = idx % matrix.cols() as usize;

                                    match matrix.get(row, col) {
                                        Some(packet) => {
                                            // Send the packet via multicast channel
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

                                            multicast_tx
                                                .send(wrapper.encode_to_vec())
                                                .expect("Failed to send multicast channel");
                                        }
                                        None => {
                                            error!(
                                                "  Packet {} not found in matrix {}, skipping.",
                                                idx, matrix_id
                                            );
                                        }
                                    }
                                }
                                SendAction::Xor(i, j) => {
                                    debug!("  Multicast packet {}⊕{}", i, j);

                                    // Get the packets for i and j
                                    let (row_i, col_i) =
                                        (i / matrix.cols() as usize, i % matrix.cols() as usize);
                                    let (row_j, col_j) =
                                        (j / matrix.cols() as usize, j % matrix.cols() as usize);

                                    let mut packet_i = match matrix.get(row_i, col_i) {
                                        Some(p) => p.encode_to_vec(),
                                        None => {
                                            error!(
                                                "  Packet {} not found in matrix {}, skipping.",
                                                i, matrix_id
                                            );
                                            continue;
                                        }
                                    };

                                    let mut packet_j = match matrix.get(row_j, col_j) {
                                        Some(p) => p.encode_to_vec(),
                                        None => {
                                            error!(
                                                "  Packet {} not found in matrix {}, skipping.",
                                                j, matrix_id
                                            );
                                            continue;
                                        }
                                    };

                                    // Use the max length of the two packets for padding
                                    let max_len = packet_i.len().max(packet_j.len());
                                    let shard_size = next_multiple_of_64(max_len as u32);

                                    info!("Shard size for Reed-Solomon encoding: {}", shard_size);
                                    padding::pad(&mut packet_i, shard_size as usize);
                                    padding::pad(&mut packet_j, shard_size as usize);

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

                                    // Encode the packets using Reed-Solomon
                                    if let Err(e) = encoder.add_original_shard(packet_i) {
                                        error!("Failed to add shard i: {}", e);
                                        continue;
                                    }
                                    if let Err(e) = encoder.add_original_shard(packet_j) {
                                        error!("Failed to add shard j: {}", e);
                                        continue;
                                    }

                                    let result = match encoder.encode() {
                                        Ok(r) => r,
                                        Err(e) => {
                                            error!(
                                                "Failed to encode packets with Reed-Solomon: {}",
                                                e
                                            );
                                            continue;
                                        }
                                    };

                                    let recovery_shard = match result.recovery_iter().next() {
                                        Some(s) => s.to_vec(),
                                        None => {
                                            error!("No recovery shard produced by Reed-Solomon encoder.");
                                            continue;
                                        }
                                    };

                                    let mut xor_packet = XorPacket {
                                        matrix: matrix_id as u32,
                                        indexes: vec![i.try_into().unwrap(), j.try_into().unwrap()],
                                        payload: recovery_shard,
                                        packet_signature: vec![], // Will be filled below
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

                                    if let Err(e) = multicast_tx.send(wrapper.encode_to_vec()) {
                                        error!("Failed to send multicast channel: {}", e);
                                    }
                                }
                            }
                            // TODO: multicast_tx.send(...)
                        }
                    }
                }
                Phase::Unicast => {} // Add other phases here as needed
            }

            self.advance_phase();
        }
    }
}
