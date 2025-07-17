use crossbeam_channel::{tick, Receiver, Sender};
use iris_shared::padding;
use log::{debug, error, info};
use openssl::ec::{EcKey, EcKeyRef};
use openssl::pkey::{Private, Public};
use openssl::sha::sha256;
use openssl::x509::X509;
use prost::Message;

use std::collections::VecDeque;
use std::error::Error;
use std::io::Cursor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use grid::Grid as Matrix;

use openssl::ecdsa::EcdsaSig;

use iris_shared::crypto_utils::decrypt;

use crate::proto::packet_wrapper::PacketType;
use crate::proto::retransmission_packet::RetransmissionType;
use crate::proto::{
    MatrixReportBitmap, PacketWrapper, ReportMetrics, RequestKey, SessionKey, TransmissionPacket,
    VideoData, XorPacket,
};
use crate::receiver::{ReceiverType, RetransmissionCounters};
use crate::CHANNEL;
use crate::SESSION;

use super::proto::{key_service_client::KeyServiceClient, ClientReport};

use tokio::runtime::Runtime;

/// Update matrix trackers in the buffer with appropriate keys based on epoch logic
fn update_buffer_matrices_with_keys(
    buffer: &mut VecDeque<MatrixTracker>,
    current_key: &SessionKey,
    next_key: &Option<SessionKey>,
) {
    let session_config = SESSION.get().unwrap().read().unwrap();
    let epoch = session_config.epoch;

    for tracker in buffer.iter_mut() {
        // Determine which key this matrix should use based on epoch logic
        if tracker.matrix_index % epoch == 0 && next_key.is_some() {
            // This matrix is at an epoch boundary, it should use the next key
            tracker.session_key = next_key.clone();
            info!(
                "Updated matrix {} (epoch boundary) with next key",
                tracker.matrix_index
            );
        } else {
            // Regular matrix, uses current key
            tracker.session_key = Some(current_key.clone());
            info!("Updated matrix {} with current key", tracker.matrix_index);
        }
    }
}

#[derive(Debug)]
pub enum TickType {
    Report,
    DecodeToApp,
}

#[derive(Default, Debug)]
/// A struct to track the state of a matrix.
struct MatrixTracker {
    /// Index of the matrix.
    matrix_index: u32,

    /// Number of columns in the matrix.
    cols: u32,

    /// Number of rows in the matrix.
    rows: u32,

    /// Number of columns for Forward Error Correction.
    cols_fec: u32,

    /// Number of rows for Forward Error Correction.
    rows_fec: u32,

    /// Matrix to store the packets.
    matrix_packets: Matrix<Option<TransmissionPacket>>,

    /// Session key to decrypt the matrix.
    session_key: Option<SessionKey>,

    /// Bitmap to track the presence of packets.
    bitmap: Vec<u8>,
}

impl MatrixTracker {
    /// Initialize the MatrixTracker with the packet's matrix details.
    fn new(packet: &TransmissionPacket) -> Self {
        let mut tracker = Self {
            matrix_index: packet.matrix,
            cols: packet.cols,
            rows: packet.rows,
            cols_fec: packet.cols_fec,
            rows_fec: packet.rows_fec,
            session_key: None,
            matrix_packets: Matrix::new(
                (packet.rows + packet.cols_fec) as usize,
                (packet.cols + packet.rows_fec) as usize,
            ),
            bitmap: vec![
                0;
                ((packet.rows + packet.cols_fec) * (packet.cols + packet.rows_fec))
                    as usize
            ], // Initialize the bitmap with 0s
        };

        // Add the initial packet to the tracker.
        tracker.add_packet(packet);

        // Return the tracker.
        tracker
    }

    /// Add packet to Tracker
    fn add_packet(&mut self, packet: &TransmissionPacket) {
        let index = packet.index as usize;
        let row = index / (self.cols + self.rows_fec) as usize;
        let col = index % (self.cols + self.rows_fec) as usize;

        if self.matrix_packets[(row, col)].is_some() {
            error!(
                "Packet already exists in the matrix at index: {} - {}",
                row, col
            );
            return;
        }

        // Add the packet to the matrix
        self.matrix_packets[(row, col)] = Some(packet.clone());

        // Update the bitmap if this is a data packet
        if (packet.index as usize) < self.bitmap.len() {
            self.bitmap[packet.index as usize] = 1;
        }
    }

    /// Returns a reference to the packet at the given (row, col) if it exists.
    pub fn get_packet(&self, row: usize, col: usize) -> Option<&TransmissionPacket> {
        // Try to get the Option<TransmissionPacket> from the matrix
        self.matrix_packets
            .get(row, col)
            .and_then(|opt_packet| opt_packet.as_ref())
    }

    /// Reconstruct the matrix using the Reed-Solomon algorithm.
    fn reconstruct_matrix(&mut self, ec_client_key: &EcKey<Private>) -> Result<(), Box<dyn Error>> {
        // This let was do to a refactor, can be improved by replacing self in all places of this function
        let tracker = self;

        let mut updates = true;

        // Logic to reconstruct the matrix
        while updates {
            let mut new_packets_added = false;

            // Reconstruct rows
            for row_index in 0..tracker.matrix_packets.rows() {
                let current_row = tracker.matrix_packets.iter_row(row_index);

                let original_shards: Vec<(usize, &Vec<u8>)> = current_row
                    .clone() // Clone the iterator to avoid consuming it
                    .take(tracker.cols as usize)
                    .enumerate()
                    .filter_map(|(i, opt_packet)| {
                        opt_packet.as_ref().map(|packet| (i, &packet.payload))
                    })
                    .collect();

                let recovery_shards: Vec<(usize, &Vec<u8>)> = current_row
                    .skip(tracker.cols as usize)
                    .enumerate()
                    .filter_map(|(i, opt_packet)| {
                        opt_packet.as_ref().map(|packet| (i, &packet.payload))
                    })
                    .collect();

                // Reed-Solomon decode the row
                if let Ok(recovered) = reed_solomon_16::decode(
                    tracker.cols as usize,     // total number of original shards
                    tracker.rows_fec as usize, // total number of recovery shards
                    original_shards,
                    recovery_shards,
                ) {
                    for (position, data) in recovered {
                        let index =
                            (tracker.cols + tracker.rows_fec) * row_index as u32 + position as u32;
                        let mut packet = TransmissionPacket {
                            matrix: tracker.matrix_index,
                            index,
                            rows: tracker.rows,
                            cols: tracker.cols,
                            rows_fec: tracker.rows_fec,
                            cols_fec: tracker.cols_fec,
                            payload: data.clone(),
                            packet_signature: vec![0, 255], // Initial empty signature
                        };

                        // Encode the packet to a vector (assuming protobuf or similar)
                        let encoded_packet = packet.encode_to_vec(); // Replace with the actual encoding method you're using
                        let message_hash = sha256(&encoded_packet);

                        // Sign the message hash
                        let signature = EcdsaSig::sign(&message_hash, ec_client_key)
                            .unwrap()
                            .to_der()
                            .unwrap();

                        // Update the packet signature
                        packet.packet_signature = signature;

                        tracker.add_packet(&packet);
                        new_packets_added = true;
                    }
                }
            }

            // Reconstruct collumn
            for col_index in 0..tracker.matrix_packets.cols() {
                let current_col = tracker.matrix_packets.iter_col(col_index);

                let original_shards: Vec<(usize, &Vec<u8>)> = current_col
                    .clone() // Clone the iterator to avoid consuming it
                    .take(tracker.rows as usize)
                    .enumerate()
                    .filter_map(|(i, opt_packet)| {
                        opt_packet.as_ref().map(|packet| (i, &packet.payload))
                    })
                    .collect();

                let recovery_shards: Vec<(usize, &Vec<u8>)> = current_col
                    .skip(tracker.rows as usize)
                    .enumerate()
                    .filter_map(|(i, opt_packet)| {
                        opt_packet.as_ref().map(|packet| (i, &packet.payload))
                    })
                    .collect();

                // Reed-Solomon decode the column
                if let Ok(recovered) = reed_solomon_16::decode(
                    tracker.rows as usize,     // total number of original shards
                    tracker.cols_fec as usize, // total number of recovery shards
                    original_shards,
                    recovery_shards,
                ) {
                    for (position, data) in recovered {
                        let index =
                            position as u32 * (tracker.cols + tracker.rows_fec) + col_index as u32;
                        let mut packet = TransmissionPacket {
                            matrix: tracker.matrix_index,
                            index,
                            rows: tracker.rows,
                            cols: tracker.cols,
                            rows_fec: tracker.rows_fec,
                            cols_fec: tracker.cols_fec,
                            payload: data.clone(),
                            packet_signature: vec![0, 255],
                        };

                        // Encode the packet to a vector (assuming protobuf or similar)
                        let encoded_packet = packet.encode_to_vec(); // Replace with the actual encoding method you're using
                        let message_hash = sha256(&encoded_packet);

                        // Sign the message hash with the client private key
                        let signature = EcdsaSig::sign(&message_hash, ec_client_key)
                            .unwrap()
                            .to_der()
                            .unwrap();

                        // Update the packet signature
                        packet.packet_signature = signature;

                        tracker.add_packet(&packet);

                        new_packets_added = true;
                    }
                }
            }

            updates = new_packets_added;
        }

        Ok(())
    }

    /// Checks if matrix is complete or not
    fn _is_complete(&self) -> bool {
        for item in self.matrix_packets.iter() {
            if item.is_none() {
                return false;
            }
        }
        true
    }

    /// Returns the missing index (the index are in row major) as vector
    fn missing_indexes(&self) -> Vec<i32> {
        let mut missing = Vec::new();
        for (index, value) in self.matrix_packets.iter().enumerate() {
            if value.is_none() {
                missing.push(index as i32);
            }
        }
        missing
    }
}

/// Validate the packet signature and return true if valid, false otherwise.
fn validate_packet(packet: &mut TransmissionPacket, x509_public_key: &EcKey<Public>) -> bool {
    // Get the signature from the packet and put the packet_signature field with 0's to calculate local hash computation
    let signature = match EcdsaSig::from_der(&packet.packet_signature) {
        Ok(sig) => sig,
        Err(err) => {
            error!("Invalid signature format: {}", err);
            return false;
        }
    };

    // Set the packet_signature field to 0's to calculate the hash
    packet.packet_signature = vec![0, 255];
    let message_hash = sha256(&packet.encode_to_vec());

    // Verify the signature
    if let Err(err) = signature.verify(&message_hash, &x509_public_key) {
        error!("Signature verification failed: {}", err);
        return false;
    }

    // Validate the packet
    true
}

fn validate_xor_packet(xor_packet: &mut XorPacket, x509_public_key: &EcKey<Public>) -> bool {
    // Get the signature from the packet and put the packet_signature field with 0's to calculate local hash computation
    let signature = match EcdsaSig::from_der(&xor_packet.packet_signature) {
        Ok(sig) => sig,
        Err(err) => {
            error!("Invalid signature format: {}", err);
            return false;
        }
    };

    // Set the packet_signature field to 0's to calculate the hash
    xor_packet.packet_signature = vec![0, 255];
    let message_hash = sha256(&xor_packet.encode_to_vec());

    // Verify the signature
    if let Err(err) = signature.verify(&message_hash, &x509_public_key) {
        error!("Signature verification failed: {}", err);
        return false;
    }

    // Validate the packet
    true
}

fn process_xor_packet(packet: &mut XorPacket, buffer: &mut VecDeque<MatrixTracker>) {
    // Find the matrix tracker for this packet
    for tracker in buffer.iter_mut() {
        if packet.matrix == tracker.matrix_index {
            // Get the indexes of the packet
            if packet.indexes.len() != 2 {
                error!(
                    "XorPacket must have exactly two indexes, found: {}",
                    packet.indexes.len()
                );
                continue;
            }

            let i = packet.indexes[0] as usize;
            let j = packet.indexes[1] as usize;

            // info!("Received XorPacket for matrix: {} indexes: ({}, {})", packet.matrix, i, j);

            let (row_i, col_i) = (i / tracker.cols as usize, i % tracker.cols as usize);
            let (row_j, col_j) = (j / tracker.cols as usize, j % tracker.cols as usize);

            // Get the packets at the indexes
            let mut original_shards = Vec::new();

            // Get the first packet
            if let Some(packet) = tracker.get_packet(row_i, col_i) {
                original_shards.push((0, packet.encode_to_vec()));
            };

            // Get the second packet
            if let Some(packet) = tracker.get_packet(row_j, col_j) {
                original_shards.push((1, packet.encode_to_vec()));
            };

            if original_shards.len() != 1 {
                error!(
                    "XorPacket must have exactly one original shard, found: {}",
                    original_shards.len()
                );
                continue;
            }

            let target_size =
                iris_shared::next_multiple_of_64(original_shards[0].1.len() as u32) as usize;

            // Pad the original shards to the target size
            for (_pos, data) in original_shards.iter_mut() {
                padding::pad(data, target_size);
            }

            // One of the packets is missing, so we can recover it
            if original_shards.len() == 1 {
                let recovery_shards = vec![(0, &packet.payload)];
                if let Ok(recovered) = reed_solomon_16::decode(
                    packet.indexes.len(), // total number of original shards
                    1,                    // total number of recovery shards
                    original_shards,
                    recovery_shards,
                ) {
                    for (_pos, mut data) in recovered {
                        iris_shared::padding::depad(&mut data).unwrap();

                        // Create a new TransmissionPacket with the recovered data
                        if let Ok(transmission) = TransmissionPacket::decode(&*data) {
                            // No need to verify the signature again, as it was already done in validate_xor_packet

                            // info!("Recovered packet: {:?}", transmission.index);
                            // Process the packet if valid
                            tracker.add_packet(&transmission);
                        }
                    }
                }
            } else {
                error!("XorPacket must have exactly one original shard");
                continue;
            }
        }
    }
}

/// Process a packet and check if it belongs to a new matrix or an old matrix in the buffer.
fn process_packet(
    buffer: &mut VecDeque<MatrixTracker>,
    packet: TransmissionPacket,
    current_key: &mut SessionKey,
    next_key: &mut Option<SessionKey>,
) {
    let session_config = SESSION.get().unwrap().read().unwrap();

    // Check if the buffer is empty
    if buffer.is_empty() {
        // Create a new tracker for the packet
        let mut matrix = MatrixTracker::new(&packet);
        // Check if the matrix index is a multiple of the epoch and rotate the key if necessary
        check_key_rotation(&mut matrix, &packet, current_key, next_key);
        // Set the session key to the matrix
        matrix.session_key = Some(current_key.clone());
        // Add the matrix to the buffer
        buffer.push_back(matrix);
        return;
    }

    // Check if the packet's matrix is already in the buffer
    if let Some(tracker) = buffer.back_mut() {
        // Check if the packet's matrix matches the tracker
        if packet.matrix == tracker.matrix_index {
            tracker.add_packet(&packet);
        } else {
            // The packet's matrix doesn't match the last tracker in the buffer

            // At this point, should be a new tracker in the buffer so try to reconstruct the matrix before with FEC
            tracker
                .reconstruct_matrix(&session_config.client_ec_key.as_ref().unwrap())
                .unwrap();

            // Check if the last tracker matrix index is less than the packet matrix index
            if tracker.matrix_index < packet.matrix {
                // The packet's matrix index is greater than the last tracker matrix index
                // This means the packet is from a new matrix

                // Create a new tracker for the packet
                let mut matrix = MatrixTracker::new(&packet);

                // Check if the matrix index is a multiple of the epoch and rotate the key if necessary
                check_key_rotation(&mut matrix, &packet, current_key, next_key);

                // Set the session key to the matrix
                matrix.session_key = Some(current_key.clone());

                // Add the matrix to the buffer
                buffer.push_back(matrix);
            } else {
                // The packet's matrix index is less than the last tracker matrix index
                // This means the packet is from an old matrix

                // Iterate over the trackers in the buffer in reverse order to find the correct tracker
                for tracker in buffer.iter_mut().rev() {
                    if tracker.matrix_index == packet.matrix {
                        tracker.add_packet(&packet);
                        break;
                    }
                }
            }
        }
    }
}

/// Process a retransmission packet and check if it belongs to a matrix in the buffer.
fn process_retransmission_packet(buffer: &mut VecDeque<MatrixTracker>, packet: TransmissionPacket) {
    // Loop through the buffer to find the matrix tracker that matches the packet's matrix index
    for tracker in buffer.iter_mut() {
        if packet.matrix == tracker.matrix_index {
            // log::debug!("Matrix index: {} - Packet index: {}", tracker.matrix_index, packet.index);
            // log::debug!("tracker before adding packet: {:?}", tracker.bitmap);
            tracker.add_packet(&packet);
            // log::debug!("tracker after adding packet: {:?}", tracker.bitmap);
        }
    }
}

/// Check if the key rotation is necessary and rotate the key if necessary. Only called when the tracker is created.
fn check_key_rotation(
    matrix: &mut MatrixTracker,
    packet: &TransmissionPacket,
    current_key: &mut SessionKey,
    next_key: &mut Option<SessionKey>,
) {
    // Get the session config
    let session_config = SESSION.get().unwrap().read().unwrap();

    // Check if the matrix index is a multiple of the epoch
    if matrix.matrix_index % session_config.epoch == 0 {
        // Check if next_key is not None
        if next_key.is_none() {
            error!("Next key is not set.");
            return;
        }

        // Get the next key from the Option and unwrap it safely
        let new_key = match next_key {
            Some(next_key_value) => {
                // remove padding from payload
                let deppaded_payload = padding::depad(&packet.payload).unwrap();

                // IMPORTANT TO SOLVE, WHEN CLIENT IS WAITING FOR MATRIX DATA AND IT HAS THE KEY, WHEN DATA ARRIVES IT ALREADY OUT OF DATE

                // Decrypt the matrix with the next_key, because the matrix is from the next epoch
                let decrypted_plaintext =
                    decrypt(&deppaded_payload, &next_key_value.key, &next_key_value.iv).unwrap();

                if decrypted_plaintext.is_empty() {
                    error!("Decryption failed or empty payload.");
                    return;
                }

                // Decode the decrypted payload as VideoData
                if let Ok(decoded) = VideoData::decode(&mut Cursor::new(decrypted_plaintext)) {
                    decoded.next_key
                } else {
                    error!("Failed to decode payload.");
                    return;
                }
            }
            None => {
                return;
            }
        };

        // Check if the new key is not None
        match new_key {
            Some(new_key_value) => {
                // Rotate the key
                rotate_key(current_key, next_key, new_key_value);
            }
            None => {
                error!("New key is not set.");
                return;
            }
        }
    }
}

/// Rotate the current key and next key.
fn rotate_key(
    current_key: &mut SessionKey,
    next_key: &mut Option<SessionKey>,
    new_key: SessionKey,
) {
    *current_key = next_key.take().unwrap();
    *next_key = Some(new_key);
}

pub fn start(
    data_receiver: Receiver<(ReceiverType, Vec<u8>)>,
    decode_data_sender: Sender<Vec<u8>>,
    client_report_sender: Sender<ClientReport>,
    shutdown_flag: Arc<AtomicBool>,
) {
    let mut buffer: VecDeque<MatrixTracker> = VecDeque::new();

    let session_config = SESSION.get().unwrap().read().unwrap();

    // Public Key from Server to verify packets signature
    let x509_public_key = X509::from_pem(&session_config.ec_server_certificate)
        .unwrap()
        .public_key()
        .unwrap()
        .ec_key()
        .unwrap();

    // Current encryption key and Option for next encryption key
    let mut current_key_session = session_config.current_key.clone();
    let mut next_key_session: Option<SessionKey> = Some(session_config.next_key.clone());

    // Client EC Key
    let client_ec_key = if let Some(ec_key) = &session_config.client_ec_key {
        ec_key
    } else {
        panic!("Client EC Key is not set.");
    };

    let mut report_counter = 1;
    let client_id = sha256(&client_ec_key.public_key_to_der().unwrap()).to_vec();

    let rint = Duration::from_millis(session_config.client_reporting_interval as u64); // interval between retries
    let max_rounds = session_config.client_sliding_window;

    let retry_tick = tick(rint);
    let mut attempt = 0;

    let mut counters = RetransmissionCounters::default();

    loop {
        // Check for shutdown signal first
        if shutdown_flag.load(Ordering::Relaxed) {
            info!("Decoder shutting down gracefully");
            break;
        }

        crossbeam_channel::select_biased! {
            recv(retry_tick) -> _ => {

                let tick = if attempt == max_rounds {
                    attempt = 0; // Reset attempt after reaching max rounds
                    TickType::DecodeToApp
                } else {
                    attempt += 1; // Increment attempt
                    TickType::Report
                };

                // Match the tick type
                match tick {
                    TickType::Report => {

                        // Check if the buffer has more than one item
                        if buffer.len() < 1 {
                            debug!("Buffer is empty or has only one item, skipping report.");
                            continue;
                        }

                        // Create a new ClientReport
                        let mut report = ClientReport {
                            client_id: client_id.clone(),
                            counter: report_counter.clone(),
                            metrics: Some(ReportMetrics {
                                multicast_retransmissions: counters.get(ReceiverType::Multicast) as u32,
                                unicast_retransmissions: counters.get(ReceiverType::Unicast) as u32,
                                wifi_aware_retransmissions: counters.get(ReceiverType::WifiAware) as u32,
                            }),
                            reports: Vec::with_capacity(buffer.len() - 1),
                            signature: vec![0, 255],
                        };

                        // Iterate over the buffer and collect reports
                        for item in buffer.iter().take(buffer.len() - 1) {
                            report.reports.push(MatrixReportBitmap {
                                matrix_index: item.matrix_index,
                                bitmap: item.bitmap.clone(),
                            });
                        }

                        // Sign the report
                        match EcdsaSig::sign(&sha256(&report.encode_to_vec()), &client_ec_key) {
                            Ok(sig) => {
                                match sig.to_der() {
                                    Ok(der) => report.signature = der,
                                    Err(e) => {
                                        error!("Failed to convert signature to DER: {}. Skipping report.", e);
                                        continue;
                                    }
                                }
                            },
                            Err(e) => {
                                error!("Failed to sign report: {}. Skipping report.", e);
                                continue;
                            }
                        }

                        // Send the report
                        if let Err(e) = client_report_sender.send(report) {
                            error!("Failed to send client report: {}. Decoder shutting down.", e);
                            break;
                        }

                        // Increment the report counter
                        report_counter += 1;

                    },
                    TickType::DecodeToApp => {

                        // Check if the buffer has more than one item
                        if buffer.len() > 1 {

                            info!("Decoding to application layer...");

                            // Drain all elements except the last one
                            let drained : Vec<MatrixTracker> = buffer.drain(0..buffer.len() - 1).collect();

                            let mut any_keys_updated = false;
                            for mut item in drained {

                                // Reconstruct the matrix
                                item.reconstruct_matrix(&client_ec_key).unwrap();

                                log::debug!("Missing packets: {:?} - {:?}", item.matrix_index, item.missing_indexes());

                                // Decode the matrix to the application layer
                                match decode_to_app_layer(&mut item, &decode_data_sender, &x509_public_key) {
                                    Ok(keys_were_updated) => {
                                        if keys_were_updated {
                                            any_keys_updated = true;
                                        }
                                    },
                                    Err(e) => {
                                        error!("Failed to decode matrix {}: {}", item.matrix_index, e);
                                    }
                                }
                            }

                            // If any keys were updated during decoding, update the remaining buffer and session keys
                            if any_keys_updated {
                                info!("Keys were updated during decoding, updating remaining matrices in buffer and session keys");

                                // Get the latest keys from the global session (they were updated in request_updated_keys_sync)
                                let session_config = SESSION.get().unwrap().read().unwrap();
                                let updated_current_key = session_config.current_key.clone();
                                let updated_next_key = session_config.next_key.clone();
                                drop(session_config); // Release the lock

                                // Update the decoder loop session keys
                                current_key_session = updated_current_key.clone();
                                next_key_session = Some(updated_next_key.clone());

                                // Update all remaining matrices in the buffer with appropriate keys based on epoch
                                update_buffer_matrices_with_keys(&mut buffer, &updated_current_key, &Some(updated_next_key));
                            }

                        }

                    }
                }

            },

            // Receive data from the channel
            recv(data_receiver) -> msg => {
                // Attempt to receive data from the channel
                let (source, data) = match msg {
                    Ok(val) => val,
                    Err(err) => {
                        error!("Channel closed: {:?}", err);
                        break;
                    }
                };

                // This is for tracking if we incremented the multicast retransmissions counter
                let mut is_multicast = false;

                let wrapper: Vec<u8> = match source {
                    // This can be helpful to track metrics
                    ReceiverType::Multicast => {
                        is_multicast = true;
                        data
                    },
                    ReceiverType::Unicast => {
                        // Increment the unicast retransmissions counter
                        counters.increment(ReceiverType::Unicast);
                        data
                    },
                    ReceiverType::WifiAware => {
                        // Increment the wifi aware retransmissions counter
                        counters.increment(ReceiverType::WifiAware);
                        data
                    }
                };

                // Attempt to decode the wrapper as a PacketWrapper
                let packet_wrapper = match PacketWrapper::decode(&*wrapper) {
                    Ok(pw) => pw,
                    Err(err) => {
                        error!("Failed to decode PacketWrapper: {}", err);
                        continue;
                    }
                };

                match packet_wrapper.packet_type {
                    Some(PacketType::TransmissionPacket(mut transmission)) => {
                        // Validate the packet signature, if invalid, skip the packet
                        if validate_packet(&mut transmission, &x509_public_key) {
                            // Process the packet if valid
                            process_packet(&mut buffer, transmission, &mut current_key_session, &mut next_key_session);
                        } else {
                            // Invalid packet, skip
                            continue;
                        }
                    }
                    Some(PacketType::RetransmissionPacket(retransmission)) => {
                        // handle RetransmissionPacket
                        match retransmission.retransmission_type {

                            Some(RetransmissionType::XorPacket(mut packet)) => {

                                // Validate the packet signature, if invalid, skip the packet
                                if validate_xor_packet(&mut packet, &x509_public_key) {

                                    // Process the XOR packet
                                    process_xor_packet(&mut packet, &mut buffer);

                                } else {
                                    // Invalid packet, skip
                                    continue;
                                }
                            }
                            // This is a normal TransmissionPacket inside a RetransmissionPacket
                            Some(RetransmissionType::TransmissionPacket(mut packet)) => {
                                // info!("Received retransmission packet for matrix: {} packet {}", packet.matrix, packet.index);
                                // Validate the packet signature, if invalid, skip the packet
                                if validate_packet(&mut packet, &x509_public_key) {
                                    // debug!("Received retransmission packet for matrix: {} packet {}", packet.matrix, packet.index);
                                    // Process the packet if valid
                                    process_retransmission_packet(&mut buffer, packet);
                                } else {
                                    // Invalid packet, skip
                                    continue;
                                }
                            }
                            None => {
                                error!("RetransmissionPacket contained no retransmission type.");
                            }
                        }

                        // Check if the packet is a multicast retransmission and increment counter
                        if is_multicast {
                            // Increment the multicast retransmissions counter
                            counters.increment(ReceiverType::Multicast);
                        }

                    }
                    None => {
                        // no packet set — error or ignore
                        error!("PacketWrapper contained no packet type.");
                        continue;
                    }
                }


            }
        }
    }
}

fn decode_to_app_layer(
    tracker: &mut MatrixTracker,
    decode_data_sender: &Sender<Vec<u8>>,
    x509_public_key: &EcKeyRef<Public>,
) -> Result<bool, Box<dyn Error>> {
    // Returns true if keys were updated
    // Logic to remove FEC rows and columns
    for _ in 0..tracker.rows_fec {
        tracker.matrix_packets.pop_col();
    }

    for _ in 0..tracker.cols_fec {
        tracker.matrix_packets.pop_row();
    }

    // Ensure the session key is set
    let mut keys_were_updated = false;
    let decryption_key = match tracker.session_key {
        Some(ref key) => key.clone(),
        None => {
            error!("Session key is not set for matrix {}", tracker.matrix_index);
            return Err("Session key is not set".into());
        }
    };

    // Decrypt and decode video data
    for item in tracker.matrix_packets.iter() {
        if let Some(value) = item {
            // remove padding from payload
            let deppaded_payload = padding::depad(&value.payload).unwrap();

            // Try decryption with current key first, then request new keys if it fails
            let decrypted_plaintext = match decrypt(
                &deppaded_payload,
                &decryption_key.key,
                &decryption_key.iv,
            ) {
                Ok(plaintext) => plaintext,
                Err(err) => {
                    error!(
                        "Decryption failed: {} \n Requesting current session key from server",
                        err
                    );

                    // Try to request updated keys from the server
                    match request_updated_keys_sync() {
                        Ok((new_current_key, _new_next_key)) => {
                            info!("Successfully received updated keys from server, retrying decryption");

                            // Try decryption again with the new key
                            match decrypt(
                                &deppaded_payload,
                                &new_current_key.key,
                                &new_current_key.iv,
                            ) {
                                Ok(plaintext) => {
                                    info!("Decryption successful with updated key");

                                    // Mark that we need to update the tracker's session key
                                    tracker.session_key = Some(new_current_key);
                                    keys_were_updated = true;

                                    plaintext
                                }
                                Err(retry_err) => {
                                    error!(
                                        "Decryption failed even with updated key: {}",
                                        retry_err
                                    );
                                    continue;
                                }
                            }
                        }
                        Err(key_err) => {
                            error!("Failed to request updated keys from server: {}", key_err);
                            continue;
                        }
                    }
                }
            };

            // Check if the decrypted payload is empty
            if decrypted_plaintext.is_empty() {
                error!("Decryption failed or empty payload.");
                continue;
            }

            // Decode the decrypted payload as VideoData
            let mut decoded = VideoData::decode(&mut Cursor::new(decrypted_plaintext)).unwrap();

            // Retrive the signature from the packet and put the packet_signature field with 0's to calculate local hash computation
            let signature = EcdsaSig::from_der(&decoded.signature)?;
            decoded.signature = vec![0, 255];

            // Calculate the local hash of the decoded data
            let message_hash = sha256(&decoded.encode_to_vec());

            // Verify the signature of the data
            let is_valid = signature.verify(&message_hash, x509_public_key).unwrap();

            // If Data signature is not valid, drop and continue
            if !is_valid {
                error!("Invalid signature for matrix: {}", tracker.matrix_index);
                continue;
            } else {
                // Send the decoded data to the application layer
                if let Err(e) = decode_data_sender.send(decoded.data) {
                    error!("Failed to send decoded data: {}. Decoder shutting down.", e);
                    break;
                }
            }
        }
    }

    Ok(keys_were_updated)
}

/// Request updated keys from the server synchronously
fn request_updated_keys_sync() -> Result<(SessionKey, SessionKey), Box<dyn Error>> {
    // Get the gRPC channel
    let channel_lock = CHANNEL.get().ok_or("gRPC channel not initialized")?;
    let channel_guard = channel_lock.read().unwrap();
    let channel = channel_guard
        .as_ref()
        .ok_or("gRPC channel not connected")?
        .clone();

    // Create a runtime for this synchronous context
    let rt = Runtime::new()?;

    rt.block_on(async {
        let mut client = KeyServiceClient::new(channel);

        // Request current keys
        let response = client
            .request_current_key(RequestKey {})
            .await
            .map_err(|e| format!("gRPC request failed: {}", e))?;

        let key_response = response.into_inner();

        let current_key = key_response
            .current_key
            .ok_or("Server did not provide current key")?;
        let next_key = key_response
            .next_key
            .ok_or("Server did not provide next key")?;

        // Update the global session with the new keys
        if let Some(session) = SESSION.get() {
            let mut session_guard = session.write().unwrap();
            session_guard.current_key = current_key.clone();
            session_guard.next_key = next_key.clone();
            info!("Updated global session with new keys from server");
        }

        Ok((current_key, next_key))
    })
}
