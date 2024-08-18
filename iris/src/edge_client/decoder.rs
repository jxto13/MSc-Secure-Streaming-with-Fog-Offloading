use crossbeam_channel::{Receiver, Sender};
use log::{error, info};
use openssl::sha::sha256;
use openssl::x509::X509;
use prost::Message;

use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::io::Cursor;
use std::time::Instant;

use crate::edge_client::proto::{SessionKey, TransmitionPacket, VideoData};
use grid::Grid as Matrix;

use openssl::ecdsa::EcdsaSig;

use crate::crypto_utils::decrypt;

use super::proto::ClientReport;

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
    receiver: Receiver<Vec<u8>>,
    decode_data_sender: Sender<Vec<u8>>,
    client_report_sender: Sender<ClientReport>,
    current_key: SessionKey,
    certificate: Vec<u8>,
    buffer_size: usize,
) -> Result<(), Box<dyn Error>> {
    let mut buffer: VecDeque<MatrixTracker> = VecDeque::new();
    let x509_public_key = X509::from_pem(&certificate)?.public_key()?.ec_key().unwrap();

    let mut current_key_session = current_key;
    let mut next_key_session : Option<SessionKey> = None;

    loop {
        match receiver.try_recv() {
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
                    info!("packet matrix: {} -- {}",packet.matrix , packet.matrix % 20 == 0);
                    let decrypted_plaintext = decrypt(&packet.payload, &current_key_session.key, &current_key_session.iv);
                    let decoded = VideoData::decode_length_delimited(&mut Cursor::new(decrypted_plaintext)).unwrap();
                    next_key_session = decoded.next_key;
                }

                // Check if the packet belongs to an existing matrix in the buffer
                if let Some(tracker) = buffer.iter_mut().find(|tracker| tracker.matrix_index == packet.matrix) {
                    // Rotate the session key only if it hasn't been rotated for this matrix
                    if packet.matrix % 20 == 0 && !tracker.key_rotated {
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
                    if packet.matrix % 20 == 0 && !new_tracker.key_rotated {
                        if let Some(new_session_key) = next_key_session.take() {
                            current_key_session = new_session_key;
                            new_tracker.key_rotated = true; // Mark rotation as done for this matrix
                        }
                    }

                    // Check size of buffer, if full pop the oldest
                    if buffer.len() >= buffer_size {
                        let mut poped_matrix = buffer.pop_front().unwrap();

                        if !poped_matrix.is_complete() {
                            reconstruct_matrix(&mut poped_matrix).unwrap();
                        }

                        send_client_report_and_decode(
                            &mut poped_matrix,
                            &client_report_sender,
                            &decode_data_sender,
                        ).unwrap();
                    }

                    new_tracker.session_key = Some(current_key_session.clone());

                    // Add it to the buffer
                    buffer.push_back(new_tracker);
                }
            }
            Err(err) => {
                if !err.is_empty() {
                    error!("Receiver error: {}", err);
                }
            }
        }
    }
}

fn reconstruct_matrix(tracker: &mut MatrixTracker) -> Result<(), Box<dyn Error>> {
    // Start timing
    let start = Instant::now();
    
    // Logic to reconstruct the matrix
    // Two iteration with column recovery helps row recovery in the second iteration

    for _ in 0..2 {
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
                *elem = Some(TransmitionPacket {
                    matrix: tracker.matrix_index,
                    index,
                    rows: tracker.rows,
                    cols: tracker.cols,
                    rows_fec: tracker.rows_fec,
                    cols_fec: tracker.cols_fec,
                    payload: data.clone(),
                    packet_signature: vec![0, 255], // need to add signature of edge client
                });
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
                *elem = Some(TransmitionPacket {
                    matrix: tracker.matrix_index,
                    index,
                    rows: tracker.rows,
                    cols: tracker.cols,
                    rows_fec: tracker.rows_fec,
                    cols_fec: tracker.cols_fec,
                    payload: data.clone(),
                    packet_signature: vec![0, 255],
                });
                tracker.update_index(index);
            }
        }
    }

    // Stop timing and log the duration
    info!("reconstruct_matrix took {:?}", start.elapsed());

    Ok(())
}

fn send_client_report_and_decode(
    tracker: &mut MatrixTracker,
    client_report_sender: &Sender<ClientReport>,
    decode_data_sender: &Sender<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {

    // Logic to remove FEC rows and columns and send client report
    for _ in 0..tracker.rows_fec {
        tracker.matrix.pop_col();
    }
    for _ in 0..tracker.cols_fec {
        tracker.matrix.pop_row();
    }

    let report = ClientReport {
        index: tracker.matrix_index,
        payload: tracker.bitmap.clone(),
    };

    
    
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
            let decoded = VideoData::decode_length_delimited(&mut Cursor::new(decrypted_plaintext)).unwrap();
            decode_data_sender.send(decoded.data.clone())?;
        }
    }
    
    client_report_sender.send(report).unwrap();

    Ok(())
}
