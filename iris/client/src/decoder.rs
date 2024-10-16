use chrono::{DateTime, Utc};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use log::{error, info};
use openssl::ec::{EcKey, EcKeyRef};
use openssl::pkey::{Private, Public};
use openssl::sha::sha256;
use openssl::x509::X509;
use prost::Message;

use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::io::Cursor;
use std::time::Instant;

use proto::{SessionKey, TransmitionPacket, VideoData};
use grid::Grid as Matrix;

use openssl::ecdsa::EcdsaSig;

use iris_shared::crypto_utils::decrypt;

use crate::{proto, TIME};

use super::proto::ClientReport;
use super::utils::SessionConfig;

#[derive(Default)]
struct MatrixTracker {
    matrix_index: i32,
    received_indexes: HashSet<i32>,
    cols: i32,
    rows: i32,
    cols_fec: i32,
    rows_fec: i32,
    matrix: Matrix<Option<TransmitionPacket>>,
    session_key: Option<SessionKey>, 
    bitmap: Vec<u8>, // Bitmap to track the presence of packets
    key_rotated: bool, // New field to track if the key has been rotated for this matrix
}

impl MatrixTracker {
    fn new(packet: &TransmitionPacket) -> Self {
         // Initialize the MatrixTracker with the packet's matrix details.
         let mut tracker = Self {
            matrix_index: packet.matrix,
            cols: packet.cols,
            rows: packet.rows,
            cols_fec: packet.cols_fec,
            rows_fec: packet.rows_fec,
            session_key: None,
            received_indexes: HashSet::new(),
            matrix: Matrix::new(
                (packet.rows + packet.cols_fec) as usize,
                (packet.cols + packet.rows_fec) as usize,
            ),
            bitmap: vec![0; (packet.rows * packet.cols) as usize], // Initialize the bitmap with 0s
            key_rotated: false, // Initialize key rotation status
        };

        // Add the initial packet to the tracker.
        tracker.add_packet(packet);

        // Return the tracker.
        tracker
    }

    // Add packet to Tracker
    fn add_packet(&mut self, packet: &TransmitionPacket) {
        let index = packet.index as usize;
        self.received_indexes.insert(packet.index);
        let row = index / (self.cols + self.rows_fec) as usize;
        let col = index % (self.cols + self.rows_fec) as usize;
        self.matrix[(row, col)] = Some(packet.clone());

        // Update the bitmap if this is a data packet (not a FEC row/col)
        if row < self.rows as usize && col < self.cols as usize {
            let data_index = row * self.cols as usize + col;
            self.bitmap[data_index] = 1; // Set the corresponding position in the bitmap to 1
        }
    }

    // Checks if matrix is complete or not
    fn is_complete(&self) -> bool {
        self.received_indexes.len() as i32
            == (self.cols + self.rows_fec) * (self.rows + self.cols_fec)
    }

    // Returns the missing index (the index are in row major) as vector
    fn missing_indexes(&self) -> Vec<i32> {
        let mut missing = Vec::new();
        for (index, value) in self.matrix.iter().enumerate() {
            if value.is_none() {
                missing.push(index as i32);
            }
        }
        missing
    }

    fn update_index(&mut self, index: i32) {
        self.received_indexes.insert(index);
    }
}

pub fn start(
    multicast_receiver: Receiver<Vec<u8>>,
    decode_data_sender: Sender<Vec<u8>>,
    client_report_sender: Sender<ClientReport>,
    session_config: SessionConfig,
    buffer_size: usize,
) -> Result<(), Box<dyn Error>> {
    let mut buffer: VecDeque<MatrixTracker> = VecDeque::new();

    // Public Key from Server to verify packets signature
    let x509_public_key = X509::from_pem(&session_config.ec_server_certificate)?.public_key()?.ec_key().unwrap();

    // Current encryption key and Option for next encryption key
    let mut current_key_session = session_config.current_key;
    let mut next_key_session : Option<SessionKey> = None;

    loop {
        match multicast_receiver.try_recv() {
            Ok(matrix_cell) => {
                let mut packet = match TransmitionPacket::decode(&*matrix_cell) {
                    Ok(packet) => packet,
                    Err(err) => {
                        error!("Error decoding packet: {}", err);
                        continue;
                    }
                };

                let signature = EcdsaSig::from_der(&packet.packet_signature)?;
                packet.packet_signature = vec![0, 255];
                let message_hash = sha256(&packet.encode_to_vec());

                let is_valid = signature.verify(&message_hash, &x509_public_key).unwrap();

                // If packet signature is not valid, drop and continue
                if !is_valid {
                    continue;
                }

                // Check if next_key_session is null
                if next_key_session == None {
                    let decrypted_plaintext = decrypt(&packet.payload, &current_key_session.key, &current_key_session.iv);
                    if !decrypted_plaintext.len() != 0 {
                        let decoded = VideoData::decode_length_delimited(&mut Cursor::new(decrypted_plaintext)).unwrap();
                        next_key_session = decoded.next_key;
                    }
                }

                // Check if the packet belongs to an existing matrix in the buffer
                if let Some(tracker) = buffer.iter_mut().find(|tracker| tracker.matrix_index == packet.matrix) {
                    // Rotate the session key only if it hasn't been rotated for this matrix
                    if packet.matrix % session_config.epoch == 0 && !tracker.key_rotated {
                        if let Some(new_session_key) = next_key_session.take() {
                            current_key_session = new_session_key;
                            tracker.key_rotated = true; // Mark rotation as done for this matrix
                        }
                    }
                    
                    tracker.add_packet(&packet);
                } else {
                    // Create a new MatrixTracker
                    let mut new_tracker = MatrixTracker::new(&packet);

                    // Rotate the session key if needed for this new matrix
                    if packet.matrix % session_config.epoch == 0 && !new_tracker.key_rotated {
                        if let Some(new_session_key) = next_key_session.take() {
                            current_key_session = new_session_key;
                            new_tracker.key_rotated = true; // Mark rotation as done for this matrix
                        }
                    }

                    // Check size of buffer, if full pop the oldest
                    if buffer.len() >= buffer_size {
                        let mut poped_matrix = buffer.pop_front().unwrap();

                        if !poped_matrix.is_complete() {
                            reconstruct_matrix(&mut poped_matrix, &session_config.client_ec_key.clone().unwrap()).unwrap();
                        }

                        send_client_report_and_decode(
                            &mut poped_matrix,
                            &client_report_sender,
                            &decode_data_sender,
                            &x509_public_key
                        ).unwrap();
                    }

                    new_tracker.session_key = Some(current_key_session.clone());

                    // Add it to the buffer
                    buffer.push_back(new_tracker);
                }
            }
            Err(TryRecvError::Empty) => {
                // No message available, continue looping
            }
            Err(TryRecvError::Disconnected) => {
                // The channel is closed, continue loop
            }
        }
    }
}

fn reconstruct_matrix(tracker: &mut MatrixTracker, ec_client_key: &EcKey<Private>) -> Result<(), Box<dyn Error>> {
    // Logic to reconstruct the matrix

    let mut updates = Vec::new();

    for (row_index, row) in tracker.matrix.iter_rows().enumerate() {
        let mut original_shards = Vec::new();
        let mut recovery_shards = Vec::new();

        for (index, element) in row.enumerate() {
            if let Some(value) = element {
                if index < tracker.cols as usize {
                    original_shards.push((index, value.payload.clone()));
                } else {
                    let enumerated_value =
                        (index - tracker.cols as usize) % tracker.rows_fec as usize;
                    recovery_shards.push((enumerated_value, value.payload.clone()));
                }
            }
        }

        if let Ok(recovered) = reed_solomon_16::decode(
            tracker.cols as usize,     // total number of original shards
            tracker.rows_fec as usize, // total number of recovery shards
            original_shards,
            recovery_shards,
        ) {
            for (position, data) in recovered {
                updates.push((row_index, position, data));
            }
        }
    }

    // Apply updates for rows
    for &(row_index, position, ref data) in &updates {
        if let Some(elem) = tracker.matrix.get_mut(row_index, position) {
            let index = (tracker.cols + tracker.rows_fec) * row_index as i32 + position as i32;
            let mut packet = TransmitionPacket {
                matrix: tracker.matrix_index,
                index,
                rows: tracker.rows,
                cols: tracker.cols,
                rows_fec: tracker.rows_fec,
                cols_fec: tracker.cols_fec,
                payload: data.clone(),
                packet_signature: vec![0,255], // Initial empty signature
            };
    
            // Encode the packet to a vector (assuming protobuf or similar)
            let encoded_packet = packet.encode_to_vec();  // Replace with the actual encoding method you're using
            let message_hash = sha256(&encoded_packet);
    
            // Sign the message hash
            let signature = EcdsaSig::sign(&message_hash, ec_client_key).unwrap().to_der().unwrap();
    
            // Update the packet signature
            packet.packet_signature = signature;
    
            // Replace the element in the matrix with the updated packet
            *elem = Some(packet);
            
            tracker.update_index(index);
        }
    }

    updates.clear();

    for (col_index, col) in tracker.matrix.iter_cols().enumerate() {
        let mut original_shards = Vec::new();
        let mut recovery_shards = Vec::new();

        for (index, element) in col.enumerate() {
            if let Some(value) = element {
                if index < tracker.rows as usize {
                    original_shards.push((index, value.payload.clone()));
                } else {
                    let enumerated_value =
                        (index - tracker.rows as usize) % tracker.cols_fec as usize;
                    recovery_shards.push((enumerated_value, value.payload.clone()));
                }
            }
        }

        if let Ok(recovered) = reed_solomon_16::decode(
            tracker.rows as usize,     // total number of original shards
            tracker.cols_fec as usize, // total number of recovery shards
            original_shards,
            recovery_shards,
        ) {
            for (position, data) in recovered {
                updates.push((col_index, position, data));
            }
        }
    }

    // Apply updates for columns
    for &(col_index, position, ref data) in &updates {
        if let Some(elem) = tracker.matrix.get_mut(col_index, position) {
            let index = (tracker.rows + tracker.cols_fec) * col_index as i32 + position as i32;
            let mut packet = TransmitionPacket {
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
            let encoded_packet = packet.encode_to_vec();  // Replace with the actual encoding method you're using
            let message_hash = sha256(&encoded_packet);
    
            // Sign the message hash
            let signature = EcdsaSig::sign(&message_hash, ec_client_key).unwrap().to_der().unwrap();
    
            // Update the packet signature
            packet.packet_signature = signature;
    
            // Replace the element in the matrix with the updated packet
            *elem = Some(packet);
            
            tracker.update_index(index);
        }
    }
    
    Ok(())
}

fn send_client_report_and_decode(
    tracker: &mut MatrixTracker,
    client_report_sender: &Sender<ClientReport>,
    decode_data_sender: &Sender<Vec<u8>>,
    x509_public_key: &EcKeyRef<Public>
) -> Result<(), Box<dyn Error>> {

    // Logic to remove FEC rows and columns and send client report
    for _ in 0..tracker.rows_fec {
        tracker.matrix.pop_col();
    }
    for _ in 0..tracker.cols_fec {
        tracker.matrix.pop_row();
    }

    // let report = ClientReport {
    //     index: tracker.matrix_index,
    //     payload: tracker.bitmap.clone(),
    // };

    // Ensure the session key is set
    let session_key = match tracker.session_key {
        Some(ref key) => key,
        None => {
            error!("Session key is not set for matrix {}", tracker.matrix_index);
            return Err("Session key is not set".into());
        }
    };
    
    // Decrypt and decode video data
    for item in tracker.matrix.iter() {
        if let Some(value) = item {
            let decrypted_plaintext = decrypt(&value.payload, &session_key.key, &session_key.iv);
            let mut decoded = VideoData::decode_length_delimited(&mut Cursor::new(decrypted_plaintext)).unwrap();

            let signature = EcdsaSig::from_der(&decoded.signature)?;
            decoded.signature = vec![0, 255];
            let message_hash = sha256(&decoded.encode_to_vec());

            let is_valid = signature.verify(&message_hash, x509_public_key).unwrap();

            // If Data signature is not valid, drop and continue
            if !is_valid {
                continue;
            } else {
                decode_data_sender.send(decoded.data).unwrap();
            }
        }
    }
    
    // Sende report
    // client_report_sender.send(report).unwrap();

    Ok(())
}
