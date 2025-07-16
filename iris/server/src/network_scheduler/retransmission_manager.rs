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
use iris_shared::{next_multiple_of_64, padding};

use crossbeam_channel::{tick, Sender};
use log::{debug, error};
use openssl::ecdsa::EcdsaSig;
use openssl::sha::sha256;
use prost::Message;
use reed_solomon_16::ReedSolomonEncoder;

/// Represents the different actions we can take to help clients recover missing packets
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum SendAction {
    /// Send an individual packet by index
    Packet(usize),
    /// Send an XOR combination of two packets (enables recovery if client has one but not the other)
    Xor(usize, usize),
}

/// Represents the two phases of retransmission strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Multicast phase: Send retransmissions to all clients simultaneously
    Multicast,
    /// Unicast phase: Send targeted retransmissions to specific clients
    Unicast,
}

/// Configuration for a retransmission phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseConfig {
    /// The type of phase (Multicast or Unicast)
    pub phase: Phase,
    /// Whether this phase is currently enabled
    pub enabled: bool,
}

/// Tracks when a packet was last sent to avoid immediate re-transmission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PacketSentInfo {
    /// Which phase index this packet was sent in
    phase_index: usize,
    /// Which round this packet was sent in
    round: usize,
}

/// Key for tracking retransmission state per matrix and action
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RetransmissionKey {
    /// The matrix ID this action applies to
    matrix_id: usize,
    /// The specific action taken (Packet or Xor)
    action: SendAction,
}

/// Manages retransmission strategy across multiple phases and matrices
/// 
/// The RetransmissionManager coordinates between multicast and unicast phases,
/// tracks what has been sent to avoid duplicates, and uses network coding
/// to efficiently recover missing packets.
pub struct RetransmissionManager {
    /// Tracks what packets/actions have been sent and when
    packet_states: HashMap<RetransmissionKey, PacketSentInfo>,
    /// Configuration for each phase (order matters)
    phases: Vec<PhaseConfig>,
    /// Current phase index in the phases vector
    phase_index: usize,
    /// Current round number (increments when we cycle back to phase 0)
    round: usize,
}

impl RetransmissionManager {
    /// Creates a new RetransmissionManager with the given phase configuration
    /// 
    /// # Arguments
    /// * `phases` - Vector of phase configurations. Must not be empty.
    /// 
    /// # Panics
    /// Panics if phases vector is empty
    pub fn new(phases: Vec<PhaseConfig>) -> Self {
        assert!(!phases.is_empty());
        Self {
            packet_states: HashMap::new(),
            phases,
            phase_index: 0,
            round: 0,
        }
    }

    /// Advances to the next enabled phase, skipping disabled ones
    /// 
    /// Cycles through all phases and increments the round counter
    /// when returning to phase 0. Only stops at enabled phases.
    fn advance_phase(&mut self) {
        let num_phases = self.phases.len();
        // Try all phases to find the next enabled one
        for _ in 0..num_phases {
            self.phase_index = (self.phase_index + 1) % num_phases;
            // Increment round when we cycle back to the first phase
            if self.phase_index == 0 {
                self.round += 1;
            }
            // Stop at the first enabled phase we find
            if self.phases[self.phase_index].enabled {
                return;
            }
        }
    }

    /// Determines if a packet can be retransmitted based on when it was last sent
    /// 
    /// Prevents sending the same packet multiple times in the same phase/round
    /// to avoid wasting bandwidth.
    /// 
    /// # Arguments
    /// * `last_sent` - Information about when this packet was last sent
    /// * `current_phase_index` - Current phase index
    /// * `current_round` - Current round number
    /// 
    /// # Returns
    /// `true` if the packet can be retransmitted, `false` otherwise
    fn can_retransmit(
        &self,
        last_sent: &PacketSentInfo,
        current_phase_index: usize,
        current_round: usize,
    ) -> bool {
        // Can retransmit if we're in a different phase or round than last time
        !(last_sent.phase_index == current_phase_index && last_sent.round == current_round)
    }

    /// Creates a PacketWrapper for a regular transmission packet
    /// 
    /// Wraps a transmission packet in the retransmission packet structure
    /// for sending over the network.
    /// 
    /// # Arguments
    /// * `packet` - The transmission packet to wrap
    /// 
    /// # Returns
    /// A PacketWrapper ready for network transmission
    fn create_transmission_packet(
        &self,
        packet: &crate::proto::TransmissionPacket,
    ) -> PacketWrapper {
        PacketWrapper {
            packet_type: Some(PacketType::RetransmissionPacket(RetransmissionPacket {
                retransmission_type: Some(RetransmissionType::TransmissionPacket(packet.clone())),
            })),
        }
    }

    /// Creates a signed XOR packet for network coding retransmission
    /// 
    /// Combines two packet indices into an XOR packet that allows clients
    /// with exactly one of the two packets to recover the missing one.
    /// The packet is cryptographically signed for authenticity.
    /// 
    /// # Arguments
    /// * `matrix_id` - ID of the matrix these packets belong to
    /// * `i` - Index of the first packet
    /// * `j` - Index of the second packet  
    /// * `recovery_shard` - The Reed-Solomon encoded XOR data
    /// * `ec_key` - Private key for signing the packet
    /// 
    /// # Returns
    /// Result containing the PacketWrapper or an error message
    fn create_xor_packet(
        &self,
        matrix_id: u32,
        i: usize,
        j: usize,
        recovery_shard: Vec<u8>,
        ec_key: &openssl::ec::EcKey<openssl::pkey::Private>,
    ) -> Result<PacketWrapper, String> {
        // Create the XOR packet structure
        let mut xor_packet = XorPacket {
            matrix: matrix_id,
            indexes: vec![i as u8, j as u8],
            payload: recovery_shard,
            packet_signature: vec![], // Will be filled below
        };

        // Sign the packet for authenticity
        let message_hash = sha256(&xor_packet.encode_to_vec());
        let signature = match EcdsaSig::sign(&message_hash, ec_key) {
            Ok(sig) => sig.to_der().unwrap_or_default(),
            Err(e) => return Err(format!("Failed to sign XOR packet: {}", e)),
        };

        xor_packet.packet_signature = signature;

        // Wrap in the retransmission packet structure
        Ok(PacketWrapper {
            packet_type: Some(PacketType::RetransmissionPacket(RetransmissionPacket {
                retransmission_type: Some(RetransmissionType::XorPacket(xor_packet)),
            })),
        })
    }

    /// Encodes two packet shards using Reed-Solomon for XOR recovery
    /// 
    /// Takes two packet payloads, pads them to the same size, and creates
    /// a Reed-Solomon recovery shard that can reconstruct either packet
    /// if the other is known.
    /// 
    /// # Arguments
    /// * `shard_i` - First packet payload
    /// * `shard_j` - Second packet payload
    /// 
    /// # Returns
    /// Result containing the recovery shard or an error message
    fn encode_xor_shards(&self, shard_i: Vec<u8>, shard_j: Vec<u8>) -> Result<Vec<u8>, String> {
        // Determine the size needed (must be multiple of 64 for Reed-Solomon)
        let max_len = shard_i.len().max(shard_j.len());
        let shard_size = next_multiple_of_64(max_len as u32);

        // Pad both shards to the same size
        let mut padded_shard_i = shard_i;
        let mut padded_shard_j = shard_j;

        padding::pad(&mut padded_shard_i, shard_size as usize);
        padding::pad(&mut padded_shard_j, shard_size as usize);

        // Create Reed-Solomon encoder (2 data shards, 1 recovery shard)
        let mut encoder = ReedSolomonEncoder::new(2, 1, shard_size as usize)
            .map_err(|e| format!("Failed to create Reed-Solomon encoder: {}", e))?;

        // Add the original shards
        encoder
            .add_original_shard(padded_shard_i)
            .map_err(|e| format!("Failed to add shard i: {}", e))?;

        encoder
            .add_original_shard(padded_shard_j)
            .map_err(|e| format!("Failed to add shard j: {}", e))?;

        // Generate the recovery shard
        let recovery_shard = encoder
            .encode()
            .map_err(|e| format!("Failed to encode packets with Reed-Solomon: {}", e))?
            .recovery_iter()
            .next()
            .unwrap()
            .to_vec();

        Ok(recovery_shard)
    }

    /// Handles the multicast phase of retransmissions
    /// 
    /// In the multicast phase, we send retransmissions to all clients simultaneously.
    /// This is efficient when multiple clients are missing the same packets.
    /// Uses network coding (XOR packets) when beneficial.
    /// 
    /// # Arguments
    /// * `client_sessions` - Session manager containing client state
    /// * `multicast_tx` - Channel for sending multicast packets
    /// * `matrix_buffer` - Buffer containing the original packet matrices
    /// * `ec_key` - Private key for signing XOR packets
    fn handle_multicast_phase(
        &mut self,
        client_sessions: &Arc<ClientSessions>,
        multicast_tx: &Sender<Vec<u8>>,
        matrix_buffer: &std::sync::Arc<crate::MatrixHistory>,
        ec_key: &openssl::ec::EcKey<openssl::pkey::Private>,
    ) {
        // Get the current network coding matrix (shows what each client has)
        let nc_matrix = client_sessions.generate_nc_matrix();

        // Process each matrix independently
        for (matrix_id, (matrix, _clients_vector)) in nc_matrix {
            if matrix.is_empty() {
                println!("Matrix {} is empty, skipping.", matrix_id);
                continue;
            }

            // Find packets we've already sent for this matrix in this phase/round
            // to avoid sending duplicates
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

            let sent_packets = compute_retransmissions(&matrix, &already_sent);

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
                        self.handle_transmission_packet(
                            idx,
                            matrix_id,
                            &buffer_matrix,
                            client_sessions,
                            multicast_tx,
                        );
                    }
                    SendAction::Xor(i, j) => {
                        self.handle_xor_packet(
                            i,
                            j,
                            matrix_id,
                            &buffer_matrix,
                            client_sessions,
                            multicast_tx,
                            ec_key,
                        );
                    }
                }
            }
        }
    }

    fn handle_transmission_packet(
        &self,
        idx: usize,
        matrix_id: u32,
        buffer_matrix: &grid::Grid<crate::proto::TransmissionPacket>,
        multicast_bandwidth_manager: &crate::membership_manager::client_sessions::ClientSessions,
        tx: &Sender<Vec<u8>>,
    ) {
        let row = idx / buffer_matrix.cols() as usize;
        let col = idx % buffer_matrix.cols() as usize;

        if let Some(packet) = buffer_matrix.get(row, col) {
            let wrapper = self.create_transmission_packet(packet);
            let wrapper_vec = wrapper.encode_to_vec();

            let can_send = multicast_bandwidth_manager
                .mutlicast_bandwidth_manager
                .try_send(wrapper_vec.len());
            if !can_send {
                debug!("Cannot send packet {}, bandwidth limit reached.", idx);
                return;
            }

            if let Err(e) = tx.send(wrapper_vec) {
                error!("Failed to send multicast packet: {}", e);
            }
        } else {
            error!(
                "Packet {} not found in matrix {}, skipping.",
                idx, matrix_id
            );
        }
    }

    fn handle_xor_packet(
        &self,
        i: usize,
        j: usize,
        matrix_id: u32,
        buffer_matrix: &grid::Grid<crate::proto::TransmissionPacket>,
        multicast_bandwidth_manager: &crate::membership_manager::client_sessions::ClientSessions,
        tx: &Sender<Vec<u8>>,
        ec_key: &openssl::ec::EcKey<openssl::pkey::Private>,
    ) {
        let shard_i = match buffer_matrix.get(
            i / buffer_matrix.cols() as usize,
            i % buffer_matrix.cols() as usize,
        ) {
            Some(p) => p.encode_to_vec(),
            None => {
                error!("Packet {} not found in matrix {}, skipping.", i, matrix_id);
                return;
            }
        };

        let shard_j = match buffer_matrix.get(
            j / buffer_matrix.cols() as usize,
            j % buffer_matrix.cols() as usize,
        ) {
            Some(p) => p.encode_to_vec(),
            None => {
                error!("Packet {} not found in matrix {}, skipping.", j, matrix_id);
                return;
            }
        };

        let recovery_shard = match self.encode_xor_shards(shard_i, shard_j) {
            Ok(shard) => shard,
            Err(e) => {
                error!("{}", e);
                return;
            }
        };

        let wrapper = match self.create_xor_packet(matrix_id, i, j, recovery_shard, ec_key) {
            Ok(w) => w,
            Err(e) => {
                error!("{}", e);
                return;
            }
        };

        let wrapper_vec = wrapper.encode_to_vec();
        let can_send = multicast_bandwidth_manager
            .mutlicast_bandwidth_manager
            .try_send(wrapper_vec.len());

        if !can_send {
            debug!(
                "Cannot send XOR packet {}, bandwidth limit reached.",
                format!("{},{}", i, j)
            );
            return;
        }

        if let Err(e) = tx.send(wrapper_vec) {
            error!("Failed to send multicast channel: {}", e);
        }
    }

    fn handle_unicast_phase(
        &mut self,
        client_sessions: &Arc<ClientSessions>,
        unicast_tx: &Sender<(Vec<u8>, IpAddr)>,
        matrix_buffer: &std::sync::Arc<crate::MatrixHistory>,
    ) {
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

                self.handle_unicast_packets_for_client(
                    missing_indices,
                    client_addr,
                    matrix_id,
                    &buffer_matrix,
                    client_sessions,
                    unicast_tx,
                );
            }
        }
    }

    fn handle_unicast_packets_for_client(
        &mut self,
        missing_indices: Vec<usize>,
        client_addr: &(Vec<u8>, IpAddr),
        matrix_id: u32,
        buffer_matrix: &grid::Grid<crate::proto::TransmissionPacket>,
        client_sessions: &Arc<ClientSessions>,
        unicast_tx: &Sender<(Vec<u8>, IpAddr)>,
    ) {
        for packet_idx in missing_indices {
            let action = SendAction::Packet(packet_idx);
            let key = RetransmissionKey {
                matrix_id: matrix_id.try_into().unwrap(),
                action: action.clone(),
            };

            let should_send = match self.packet_states.get(&key) {
                Some(info) => self.can_retransmit(info, self.phase_index, self.round),
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

            let row = packet_idx / buffer_matrix.cols() as usize;
            let col = packet_idx % buffer_matrix.cols() as usize;

            if let Some(packet) = buffer_matrix.get(row, col) {
                let wrapper = self.create_transmission_packet(packet);
                let wrapper_vec = wrapper.encode_to_vec();

                let client_entry = client_sessions.get_client_data(&client_addr.0);
                if client_entry.is_none() {
                    error!("Client data for {:?} not found, skipping.", client_addr);
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

                if let Err(e) = unicast_tx.send((wrapper_vec, client_addr.1)) {
                    error!("Failed to send unicast packet to {:?}: {}", client_addr, e);
                }
            } else {
                error!(
                    "Packet {} not found in matrix {}, skipping.",
                    packet_idx, matrix_id
                );
            }
        }
    }

    /// Main entry point for handling retransmissions across all phases
    /// 
    /// This method runs continuously, cycling through enabled phases and
    /// sending retransmissions based on client feedback. It coordinates
    /// between multicast and unicast phases according to the configured
    /// phase schedule.
    /// 
    /// # Arguments
    /// * `client_sessions` - Session manager containing client state and feedback
    /// * `multicast_tx` - Channel for sending multicast retransmissions
    /// * `unicast_tx` - Channel for sending unicast retransmissions
    pub fn handle_retransmissions(
        &mut self,
        client_sessions: Arc<ClientSessions>,
        multicast_tx: Sender<Vec<u8>>,
        unicast_tx: Sender<(Vec<u8>, IpAddr)>,
    ) {
        // Get global configuration and resources
        let config = CONFIG.get().unwrap();
        let matrix_buffer = MATRIX_BUFFER.get().unwrap();
        let manager = MANAGER.get().unwrap();

        // Get the private key for signing XOR packets
        let ec_key = {
            let binding = manager.read().unwrap();
            binding.get_ec_key().clone()
        };

        // Create a ticker for regular retransmission intervals
        let interval_ticker = tick(Duration::from_millis(config.retransmission.interval as u64));

        // Main retransmission loop
        for _ in interval_ticker {
            let phase_config = self.phases[self.phase_index];
            
            // Skip disabled phases
            if !phase_config.enabled {
                self.advance_phase();
                continue;
            }

            // Execute the appropriate phase strategy
            match phase_config.phase {
                Phase::Multicast => {
                    self.handle_multicast_phase(
                        &client_sessions,
                        &multicast_tx,
                        &matrix_buffer,
                        &ec_key,
                    );
                }
                Phase::Unicast => {
                    self.handle_unicast_phase(&client_sessions, &unicast_tx, &matrix_buffer);
                }
            }

            // Move to the next phase for the next iteration
            self.advance_phase();
        }
    }
}

/// Computes optimal retransmission actions based on client feedback
/// 
/// This function analyzes the network coding matrix (which shows what packets
/// each client has received) and determines the most efficient retransmission
/// strategy. It considers both individual packet retransmissions and network
/// coding (XOR) opportunities.
/// 
/// Key improvements over previous version:
/// - Does not modify the input matrix (preserves real client state)
/// - Properly calculates XOR benefit (only helps clients with exactly one packet)
/// - Uses bandwidth efficiently by comparing strategies
/// - Respects already_sent to avoid duplicates
/// 
/// # Arguments
/// * `network_coding_matrix` - Matrix showing packet reception status per client
///   (1 = received, 0 = missing)
/// * `already_sent` - Set of actions already taken in this phase/round
/// 
/// # Returns
/// Vector of recommended SendActions for this retransmission round
pub fn compute_retransmissions(
    network_coding_matrix: &grid::Grid<u8>,
    already_sent: &HashSet<SendAction>,
) -> Vec<SendAction> {
    let num_clients = network_coding_matrix.rows();
    let num_packets = network_coding_matrix.cols();
    let mut sent_packets = Vec::new();

    let sent_set = already_sent.clone();

    // Count missing packets per column (how many clients are missing each packet)
    let mut missing_count = vec![0; num_packets];
    for row in 0..num_clients {
        for col in 0..num_packets {
            if network_coding_matrix[(row, col)] == 0 {
                missing_count[col] += 1;
            }
        }
    }

    // If all packets are received by all clients, nothing to do
    if missing_count.iter().all(|&x| x == 0) {
        return sent_packets;
    }

    // Strategy 1: Find the most-needed individual packet
    // This is the packet missing by the most clients
    let mut max_idx = None;
    let mut max_val = 0;
    for (idx, &val) in missing_count.iter().enumerate() {
        if val > max_val && !sent_set.contains(&SendAction::Packet(idx)) {
            max_val = val;
            max_idx = Some(idx);
        }
    }

    // Strategy 2: Look for beneficial XOR opportunities
    // An XOR(i,j) helps clients that have exactly one of {i,j} but not both
    // This is more efficient than sending both packets individually
    let mut best_xor = None;
    let mut best_xor_benefit = 0;
    
    for i in 0..num_packets {
        for j in (i + 1)..num_packets {
            // Skip if we've already sent this XOR combination
            if sent_set.contains(&SendAction::Xor(i, j)) {
                continue;
            }
            
            // Count how many clients would benefit from XOR(i,j)
            let mut xor_benefit = 0;
            for row in 0..num_clients {
                let has_i = network_coding_matrix[(row, i)] == 1;
                let has_j = network_coding_matrix[(row, j)] == 1;
                
                // Client benefits if they have exactly one of the two packets
                // They can use XOR to recover the missing one
                if (has_i && !has_j) || (!has_i && has_j) {
                    xor_benefit += 1;
                }
            }
            
            // Track the best XOR opportunity
            if xor_benefit > best_xor_benefit {
                best_xor_benefit = xor_benefit;
                best_xor = Some((i, j));
            }
        }
    }

    // Strategy 3: Make the optimal decision based on effectiveness
    // Compare XOR benefit vs individual packet benefit and choose the best
    if let Some((i, j)) = best_xor {
        // Send XOR if it helps more clients than the best individual packet
        if best_xor_benefit > max_val {
            sent_packets.push(SendAction::Xor(i, j));
        } else if let Some(idx) = max_idx {
            sent_packets.push(SendAction::Packet(idx));
        }
    } else if let Some(idx) = max_idx {
        // No beneficial XOR found, send the most-needed individual packet
        sent_packets.push(SendAction::Packet(idx));
    }

    // Strategy 4: Use remaining bandwidth for additional packets
    // If we have capacity and there are more packets that would help,
    // add the second most needed packet to maximize efficiency
    if sent_packets.len() < 2 {
        let mut second_max_idx = None;
        let mut second_max_val = 0;
        for (idx, &val) in missing_count.iter().enumerate() {
            if val > second_max_val 
                && val < max_val  // Must be less than the first choice
                && !sent_set.contains(&SendAction::Packet(idx))  // Not already sent
                && !sent_packets.contains(&SendAction::Packet(idx))  // Not in current batch
            {
                second_max_val = val;
                second_max_idx = Some(idx);
            }
        }
        if let Some(idx) = second_max_idx {
            sent_packets.push(SendAction::Packet(idx));
        }
    }

    sent_packets
}
