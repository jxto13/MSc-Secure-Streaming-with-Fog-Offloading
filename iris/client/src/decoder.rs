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
    Arc, RwLock,
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
use crate::session::ClientSession;

#[derive(Debug, Clone)]
pub struct PacketRequest {
    pub matrix_index: u32,
    pub packet_index: u32,
}

#[derive(Debug, Clone)]
pub struct PacketResponse {
    pub packet: Option<TransmissionPacket>,
}
use tonic::transport::Channel;

use super::proto::{key_service_client::KeyServiceClient, ClientReport};

use tokio::runtime::Runtime;

#[derive(Debug)]
pub enum TickType {
    Report,
    DecodeToApp,
}

#[derive(Default, Debug, Clone)]
/// A struct to track the state of a matrix.
pub struct MatrixTracker {
    /// Index of the matrix.
    pub matrix_index: u32,

    /// Number of columns in the matrix.
    pub cols: u32,

    /// Number of rows in the matrix.
    pub rows: u32,

    /// Number of columns for Forward Error Correction.
    pub cols_fec: u32,

    /// Number of rows for Forward Error Correction.
    pub rows_fec: u32,

    /// Matrix to store the packets.
    pub matrix_packets: Matrix<Option<TransmissionPacket>>,

    /// Session key to decrypt the matrix.
    pub session_key: Option<SessionKey>,

    /// Bitmap to track the presence of packets.
    pub bitmap: Vec<u8>,

    /// Packet loss percentage for this matrix.
    pub packet_loss_percentage: f64,
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
            packet_loss_percentage: 0.0,
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

        // Recalculate packet loss after adding the packet
        self.calculate_packet_loss();
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

    /// Calculate and update the packet loss percentage for this matrix.
    /// Considers both data packets and FEC packets.
    fn calculate_packet_loss(&mut self) {
        let total_packets = ((self.rows + self.cols_fec) * (self.cols + self.rows_fec)) as usize;
        let mut received_packets = 0;

        // Count all received packets (both data and FEC)
        for row in 0..(self.rows + self.cols_fec) as usize {
            for col in 0..(self.cols + self.rows_fec) as usize {
                if self.matrix_packets[(row, col)].is_some() {
                    received_packets += 1;
                }
            }
        }

        let lost_packets = total_packets - received_packets;
        self.packet_loss_percentage = if total_packets > 0 {
            (lost_packets as f64 / total_packets as f64) * 100.0
        } else {
            0.0
        };
    }

    /// Get the current packet loss percentage.
    fn get_packet_loss_percentage(&self) -> f64 {
        self.packet_loss_percentage
    }
}

/// Validate the packet signature and return true if valid, false otherwise.
fn validate_packet(packet: &mut TransmissionPacket, x509_public_key: &EcKey<Public>) -> bool {
    // Save the packet signature to restore it later
    let packet_signature = packet.packet_signature.clone();

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

    packet.packet_signature = packet_signature; // Restore the original signature

    // Validate the packet
    true
}

fn validate_xor_packet(xor_packet: &mut XorPacket, x509_public_key: &EcKey<Public>) -> bool {
    // Save the packet signature to restore it later
    let packet_signature = xor_packet.packet_signature.clone();

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

    xor_packet.packet_signature = packet_signature; // Restore the original signature

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
    session: &Arc<RwLock<ClientSession>>,
) {
    let session_config = session.read().unwrap();

    // Check if the buffer is empty
    if buffer.is_empty() {
        // Create a new tracker for the packet
        let mut matrix = MatrixTracker::new(&packet);
        // Check if the matrix index is a multiple of the epoch and rotate the key if necessary
        check_key_rotation(&mut matrix, &packet, current_key, next_key, session);
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
                check_key_rotation(&mut matrix, &packet, current_key, next_key, session);

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
    session: &Arc<RwLock<ClientSession>>,
) {
    // Get the session config
    let session_config = session.read().unwrap();

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

#[derive(Debug)]
pub struct Metrics {
    pub packet_loss_percentage: f64,
}

#[derive(Debug)]
pub struct Decoder {
    data_receiver: Receiver<(ReceiverType, Vec<u8>)>,
    decoded_data_sender: Sender<Vec<u8>>,
    client_report_sender: Sender<ClientReport>,
    packet_request_receiver: Receiver<(PacketRequest, crossbeam_channel::Sender<PacketResponse>)>,
    buffer_sync_sender: Option<Sender<Vec<MatrixTracker>>>,
    shutdown_flag: Arc<AtomicBool>,
    session: Arc<RwLock<ClientSession>>,
    channel: Channel,
    metrics: Arc<RwLock<Metrics>>,
}

impl Decoder {
    pub fn new(
        data_receiver: Receiver<(ReceiverType, Vec<u8>)>,
        decoded_data_sender: Sender<Vec<u8>>,
        client_report_sender: Sender<ClientReport>,
        packet_request_receiver: Receiver<(
            PacketRequest,
            crossbeam_channel::Sender<PacketResponse>,
        )>,
        buffer_sync_sender: Option<Sender<Vec<MatrixTracker>>>,
        shutdown_flag: Arc<AtomicBool>,
        session: Arc<RwLock<ClientSession>>,
        channel: Channel,
    ) -> Self {
        Decoder {
            data_receiver,
            decoded_data_sender,
            client_report_sender,
            packet_request_receiver,
            buffer_sync_sender,
            shutdown_flag,
            session,
            channel,
            metrics: Arc::new(RwLock::new(Metrics {
                packet_loss_percentage: 0.0,
            })),
        }
    }

    pub fn start(&self) {
        let mut buffer: VecDeque<MatrixTracker> = VecDeque::new();

        let session_config = self.session.read().unwrap();

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

        use std::collections::HashMap;
        let mut last_reported_bitmaps: HashMap<u32, Vec<u8>> = HashMap::new();

        loop {
            // Check for shutdown signal first
            if self.shutdown_flag.load(Ordering::Relaxed) {
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

                            // Send buffer snapshot to retransmission manager for caching
                            if let Some(ref sync_sender) = self.buffer_sync_sender {
                                // Clone the buffer for the retransmission manager
                                let buffer_snapshot: Vec<MatrixTracker> = buffer.iter().cloned().collect();
                                if let Err(e) = sync_sender.try_send(buffer_snapshot) {
                                    debug!("Failed to send buffer snapshot to retransmission manager: {}", e);
                                }
                            }

                            // Check if the buffer has more than one item
                            if buffer.len() < 1 {
                                debug!("Buffer is empty or has only one item, skipping report.");
                                continue;
                            }


                            // Clean up last_reported_bitmaps to avoid unbounded growth
                            let buffer_indices: std::collections::HashSet<u32> = buffer.iter().take(buffer.len() - 1).map(|item| item.matrix_index).collect();
                            last_reported_bitmaps.retain(|matrix_index, _| buffer_indices.contains(matrix_index));

                            // Only report matrices whose bitmap has changed since last report
                            let mut changed_reports = Vec::new();
                            for item in buffer.iter().take(buffer.len() - 1) {
                                let matrix_index = item.matrix_index;
                                let bitmap = &item.bitmap;
                                let last_bitmap = last_reported_bitmaps.get(&matrix_index);
                                if last_bitmap.map_or(true, |b| b != bitmap) {
                                    changed_reports.push(MatrixReportBitmap {
                                        matrix_index,
                                        bitmap: bitmap.clone(),
                                    });
                                    last_reported_bitmaps.insert(matrix_index, bitmap.clone());
                                }
                            }

                            if !changed_reports.is_empty() {
                                let mut report = ClientReport {
                                    client_id: client_id.clone(),
                                    counter: report_counter.clone(),
                                    metrics: Some(ReportMetrics {
                                        multicast_retransmissions: counters.get(ReceiverType::Multicast) as u32,
                                        unicast_retransmissions: counters.get(ReceiverType::Unicast) as u32,
                                        wifi_aware_retransmissions: counters.get(ReceiverType::WifiAware) as u32,
                                    }),
                                    reports: changed_reports,
                                    signature: vec![0, 255],
                                };

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
                                if let Err(e) = self.client_report_sender.send(report) {
                                    error!("Failed to send client report: {}. Decoder shutting down.", e);
                                    break;
                                }

                                // Increment the report counter
                                report_counter += 1;
                            } else {
                                // debug!("No changed matrix bitmaps to report.");
                            }

                        },
                        TickType::DecodeToApp => {

                            // Check if the buffer has more than one item
                            if buffer.len() > 1 {

                                // info!("Decoding to application layer...");

                                // Drain all elements except the last one
                                let drained : Vec<MatrixTracker> = buffer.drain(0..buffer.len() - 1).collect();

                                // Reset the counters for the next round
                                let mut overall_loss = 0.0;

                                for mut item in drained {

                                    // Reconstruct the matrix
                                    item.reconstruct_matrix(&client_ec_key).unwrap();

                                    log::debug!("Missing packets: {:?} - {:?}", item.matrix_index, item.missing_indexes());

                                    // Calculate packet loss before reconstruction
                                    item.calculate_packet_loss();

                                    overall_loss += item.get_packet_loss_percentage();

                                    // Update the metrics
                                    self.metrics.write().unwrap().packet_loss_percentage = overall_loss;

                                    // Decode the matrix to the application layer
                                    match decode_to_app_layer(&mut item, &self.decoded_data_sender, &x509_public_key, &self.session, &self.channel) {
                                        Ok(_) => {
                                            // If decoding was successful, we can consider the keys updated
                                        },
                                        Err(e) => {
                                            error!("Failed to decode matrix {}: {}", item.matrix_index, e);
                                        }
                                    }
                                }

                            }

                        }
                    }

                },

                // Receive data from the channel
                recv(self.data_receiver) -> msg => {
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
                                process_packet(&mut buffer, transmission, &mut current_key_session, &mut next_key_session, &self.session);
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

                // Handle packet requests from RetransmissionManager
                recv(self.packet_request_receiver) -> msg => {
                    let (request, response_sender) = match msg {
                        Ok(val) => val,
                        Err(_) => continue, // Channel closed or no message
                    };

                    // Search for the packet in the buffer
                    let mut found_packet = None;
                    for tracker in buffer.iter() {
                        if tracker.matrix_index == request.matrix_index {
                            let packet_index = request.packet_index as usize;
                            let row = packet_index / (tracker.cols + tracker.rows_fec) as usize;
                            let col = packet_index % (tracker.cols + tracker.rows_fec) as usize;

                            if let Some(packet) = tracker.get_packet(row, col) {
                                found_packet = Some(packet.clone());
                                break;
                            }
                        }
                    }

                    // Send response
                    let response = PacketResponse {
                        packet: found_packet,
                    };

                    if let Err(_) = response_sender.send(response) {
                        // Receiver dropped, ignore
                    }
                }
            }
        }
    }

    pub fn get_metrics(&self) -> Arc<RwLock<Metrics>> {
        self.metrics.clone()
    }
}

fn decode_to_app_layer(
    tracker: &mut MatrixTracker,
    decode_data_sender: &Sender<Vec<u8>>,
    x509_public_key: &EcKeyRef<Public>,
    session: &Arc<RwLock<ClientSession>>,
    channel: &Channel,
) -> Result<(), Box<dyn Error>> {
    // Returns true if keys were updated
    // Logic to remove FEC rows and columns
    for _ in 0..tracker.rows_fec {
        tracker.matrix_packets.pop_col();
    }

    for _ in 0..tracker.cols_fec {
        tracker.matrix_packets.pop_row();
    }

    // Use the tracker session key for decryption unless a key update is triggered
    let decryption_key = match &tracker.session_key {
        Some(ref key) => key,
        None => {
            error!("Session key is not set for matrix {}", tracker.matrix_index);
            return Err("Session key is not set".into());
        }
    };

    // Decrypt and decode video data
    let mut used_new_keys = false;
    let mut new_current_key: Option<SessionKey> = None;
    let mut new_next_key: Option<SessionKey> = None;

    let epoch = {
        let session_guard = session.read().unwrap();
        session_guard.epoch
    };

    for item in tracker.matrix_packets.iter() {
        if let Some(value) = item {
            // remove padding from payload
            let deppaded_payload = match padding::depad(&value.payload) {
                Ok(p) => p,
                Err(_) => {
                    error!("Depadding failed for matrix: {}", tracker.matrix_index);
                    continue;
                }
            };

            // Select which key to use
            let (key, iv) = if used_new_keys {
                if tracker.matrix_index % epoch == 0 {
                    match &new_next_key {
                        Some(k) => (&k.key, &k.iv),
                        None => {
                            error!(
                                "No next key available after key update for matrix: {}",
                                tracker.matrix_index
                            );
                            continue;
                        }
                    }
                } else {
                    match &new_current_key {
                        Some(k) => (&k.key, &k.iv),
                        None => {
                            error!(
                                "No current key available after key update for matrix: {}",
                                tracker.matrix_index
                            );
                            continue;
                        }
                    }
                }
            } else {
                (&decryption_key.key, &decryption_key.iv)
            };

            let mut decrypted_plaintext = decrypt(&deppaded_payload, key, iv);

            // If decryption fails and we haven't tried new keys yet, request new keys and try again
            if decrypted_plaintext.is_err() && !used_new_keys {
                error!(
                    "Decryption failed: {}. Requesting new session keys.",
                    decrypted_plaintext.as_ref().err().unwrap()
                );
                match request_updated_keys_sync(&channel, &session) {
                    Ok((curr, next)) => {
                        let mut session_guard = session.write().unwrap();
                        session_guard.current_key = curr.clone();
                        session_guard.next_key = next.clone();
                        new_current_key = Some(curr);
                        new_next_key = Some(next);
                        used_new_keys = true;

                        // Retry decryption with the new key
                        let epoch = session_guard.epoch;
                        let (key, iv) = if tracker.matrix_index % epoch == 0 {
                            match &new_next_key {
                                Some(k) => (&k.key, &k.iv),
                                None => {
                                    error!(
                                        "No next key available after key update for matrix: {}",
                                        tracker.matrix_index
                                    );
                                    continue;
                                }
                            }
                        } else {
                            match &new_current_key {
                                Some(k) => (&k.key, &k.iv),
                                None => {
                                    error!(
                                        "No current key available after key update for matrix: {}",
                                        tracker.matrix_index
                                    );
                                    continue;
                                }
                            }
                        };
                        decrypted_plaintext = decrypt(&deppaded_payload, key, iv);
                    }
                    Err(key_err) => {
                        error!("Failed to request updated keys from server: {}", key_err);
                        continue;
                    }
                }
            }

            let decrypted_plaintext = match decrypted_plaintext {
                Ok(plaintext) => plaintext,
                Err(err) => {
                    error!("Decryption failed after key update: {}", err);
                    continue;
                }
            };

            if decrypted_plaintext.is_empty() {
                error!("Decryption failed or empty payload.");
                continue;
            }

            // Decode the decrypted payload as VideoData
            let mut decoded = match VideoData::decode(&mut Cursor::new(decrypted_plaintext)) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to decode VideoData: {}", e);
                    continue;
                }
            };

            // Retrieve the signature from the packet and put the packet_signature field with 0's to calculate local hash computation
            let signature = match EcdsaSig::from_der(&decoded.signature) {
                Ok(sig) => sig,
                Err(e) => {
                    error!("Failed to parse signature: {}", e);
                    continue;
                }
            };

            decoded.signature = vec![0, 255];

            // Calculate the local hash of the decoded data
            let message_hash = sha256(&decoded.encode_to_vec());

            // Verify the signature of the data
            let is_valid = match signature.verify(&message_hash, x509_public_key) {
                Ok(valid) => valid,
                Err(e) => {
                    error!("Signature verification error: {}", e);
                    continue;
                }
            };

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

    Ok(())
}

/// Request updated keys from the server synchronously
fn request_updated_keys_sync(
    channel: &Channel,
    session: &Arc<RwLock<ClientSession>>,
) -> Result<(SessionKey, SessionKey), Box<dyn Error>> {
    // Use the provided channel directly
    let channel_clone = channel.clone();

    // Create a runtime for this synchronous context
    let rt = Runtime::new()?;

    info!("Requesting updated keys from server...");
    rt.block_on(async {
        let mut client = KeyServiceClient::new(channel_clone);
        info!("Successfully created KeyServiceClient");

        info!("Sending request for current keys...");
        // Request current keys
        let response = client
            .request_current_key(RequestKey {})
            .await
            .map_err(|e| format!("gRPC request failed: {}", e))?;
        info!("Received response for current keys from server");
        let key_response = response.into_inner();

        let current_key = key_response
            .current_key
            .ok_or("Server did not provide current key")?;
        let next_key = key_response
            .next_key
            .ok_or("Server did not provide next key")?;

        info!(
            "Keys: current_key={:?}, next_key={:?}",
            current_key, next_key
        );
        // Update the global session with the new keys
        let mut session_guard = session.write().unwrap();
        session_guard.current_key = current_key.clone();
        session_guard.next_key = next_key.clone();
        info!("Updated global session with new keys from server");

        Ok((current_key, next_key))
    })
}
