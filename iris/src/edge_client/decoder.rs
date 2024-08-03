use crossbeam_channel::{Receiver, Sender};
use log::{error, info};
use openssl::sha::sha256;
use openssl::x509::X509;
use prost::Message;

use std::collections::HashSet;
use std::error::Error;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

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
}

impl MatrixTracker {
    fn new(packet: &TransmitionPacket) -> Self {
        Self {
            matrix_index: packet.matrix,
            received_indexes: HashSet::new(),
            cols: packet.cols,
            rows: packet.rows,
            cols_fec: packet.cols_fec,
            rows_fec: packet.rows_fec,
            matrix: Matrix::new(
                (packet.rows + packet.cols_fec) as usize,
                (packet.cols + packet.rows_fec) as usize,
            ),
        }
    }

    // Add packet to Tracker
    fn add_packet(&mut self, packet: TransmitionPacket) {
        let index = packet.index as usize;
        self.received_indexes.insert(packet.index);
        let row = index / (self.cols + self.rows_fec) as usize;
        let col = index % (self.cols + self.rows_fec) as usize;
        self.matrix[(row, col)] = Some(packet);
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
#[ignore]
pub fn start(receiver: Receiver<Vec<u8>>, decode_data_sender: Sender<Vec<u8>>, client_report_sender: tokio::sync::mpsc::Sender<ClientReport>, current_key: SessionKey, certificate: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let current_tracker: Arc<Mutex<Option<MatrixTracker>>> = Arc::new(Mutex::new(None));

    let x509_public_key = X509::from_pem(&certificate).unwrap().public_key().unwrap();

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


                let signature = EcdsaSig::from_der(&packet.packet_signature).unwrap();

                packet.packet_signature = vec![0,255];

                let message_hash = sha256(&packet.encode_to_vec());
                
                // Verify the signature
                let is_valid = signature.verify(&message_hash, &x509_public_key.ec_key().unwrap()).unwrap();
                
                if !is_valid {
                    continue;
                }

                let mut tracker_guard = current_tracker.lock().unwrap();

                if let Some(ref mut tracker) = *tracker_guard {
                    if tracker.matrix_index != packet.matrix {
                        // Check for missing packets
                        if !tracker.is_complete() {

                            // Record the start time
                            let start = std::time::Instant::now();
                            
                            // Two iteration with collumn recovery helps row recovery in the second iteration
                            for _ in 0..2 {                                
                                let mut updates = Vec::new();
                                
                                for (row_index, row) in tracker.matrix.iter_rows().enumerate() {
                                    
                                    let mut original_shards = Vec::new();
                                    let mut recovery_shards = Vec::new();

                                    for (index, element) in row.enumerate() {
                                        // check empty packet
                                        if element.is_none() {
                                            continue;
                                        }

                                        if let Some(value) = element {
                                            if index < tracker.cols as usize {
                                                original_shards.push((index, value.payload.clone()));
                                            } else {
                                                let enumerated_value = (index - tracker.cols as usize) % tracker.rows_fec as usize;
                                                recovery_shards.push((enumerated_value, value.payload.clone()));
                                            }
                                        }

                                    }

                                    match reed_solomon_16::decode(
                                        tracker.cols as usize,     // total number of original shards
                                        tracker.rows_fec as usize, // total number of recovery shards
                                        original_shards,
                                        recovery_shards,
                                    ) {
                                        Ok(recovered) => {
                                            for (position, data) in recovered {
                                                updates.push((row_index, position, data));
                                            }
                                        }
                                        Err(_) => (),
                                    }
                                }

                                // Perform the updates in a separate loop to avoid mutable and immutable borrow conflicts
                                for (row_index, position, data) in updates {
                                    if let Some(elem) = tracker.matrix.get_mut(row_index, position) {

                                        let index = ((tracker.cols + tracker.rows_fec) * row_index as i32) + position as i32;

                                        *elem = Some(TransmitionPacket { 
                                            matrix: tracker.matrix_index,
                                            index: index,
                                            rows: tracker.rows,
                                            cols: tracker.cols,
                                            rows_fec: tracker.rows_fec,
                                            cols_fec: tracker.cols_fec,
                                            payload: data,
                                            packet_signature: vec![0,255], // need to add signature of edge client
                                        });
                                        
                                        tracker.update_index(index);
                                    } else {
                                        error!("Failed to access matrix at position ({},{})", row_index, position);
                                    }
                                }

                                let mut updates = Vec::new();

                                for (col_index, col) in tracker.matrix.iter_cols().enumerate() {
                                    
                                    let mut original_shards = Vec::new();
                                    let mut recovery_shards = Vec::new();

                                    for (index, element) in col.enumerate() {
                                        // check empty packet
                                        if element.is_none() {
                                            continue;
                                        }

                                        if let Some(value) = element {
                                            if index < tracker.rows as usize {
                                                original_shards.push((index, value.payload.clone()));
                                            } else {
                                                let enumerated_value = (index - tracker.rows as usize) % tracker.cols_fec as usize;
                                                recovery_shards.push((enumerated_value, value.payload.clone()));
                                            }
                                        }

                                    }

                                    match reed_solomon_16::decode(
                                        tracker.rows as usize,     // total number of original shards
                                        tracker.cols_fec as usize, // total number of recovery shards
                                        original_shards,
                                        recovery_shards,
                                    ) {
                                        Ok(recovered) => {
                                            for (position, data) in recovered {
                                                updates.push((col_index, position, data));
                                            }
                                        }
                                        Err(_) => (),
                                    }
                                }

                                // Perform the updates in a separate loop to avoid mutable and immutable borrow conflicts
                                for (col_index, position, data) in updates {
                                    if let Some(elem) = tracker.matrix.get_mut(col_index, position) {
                                        let index = ((tracker.rows + tracker.cols_fec) * col_index as i32) + position as i32;
                                        *elem = Some(TransmitionPacket { 
                                            matrix: tracker.matrix_index,
                                            index: index,
                                            rows: tracker.rows,
                                            cols: tracker.cols,
                                            rows_fec: tracker.rows_fec,
                                            cols_fec: tracker.cols_fec,
                                            payload: data,
                                            packet_signature: vec![0,255],
                                        });
                                        tracker.update_index(index);
                                    } else {
                                        error!("Failed to access matrix at position ({},{})", col_index, position);
                                    }
                                }
                            }

                            // Print the elapsed time
                            info!(
                                "Time elapsed for reconstructing matrix {:?}: {:?}",
                                tracker.matrix_index,
                                start.elapsed(),
                            );

                        }

                        // remove the row fec
                        for _ in 0..tracker.rows_fec {
                            tracker.matrix.pop_col();
                        }

                        // remove the collumn fec
                        for _ in 0..tracker.cols_fec {
                            tracker.matrix.pop_row();
                        }
                        
                        let missing_packet = tracker.missing_indexes();

                        if missing_packet.len() != 0 {
                            info!("Missing packets for matrix {}: {:?}",tracker.matrix_index, tracker.missing_indexes());
                        }

                        // Create Bitmap for matrix
                        let mut report_bitmap = Vec::new();

                        for item in tracker.matrix.iter() {
                            if item.is_none() {
                                report_bitmap.push(0);
                            } else {
                                report_bitmap.push(1);
                            }
                        }

                        // Create ClientReport
                        let report = ClientReport {
                            index: tracker.matrix_index,
                            payload: report_bitmap,
                        };
                        
                        // Send ClientReport of matrix
                        client_report_sender.blocking_send(report).unwrap();

                        // send to the player (need to put this somewhere else)
                        for item in tracker.matrix.iter() {
                            if let Some(value) = item {
                                let decrypted_plaintext = decrypt(&value.payload, &current_key.key, &current_key.iv);

                                let decoded = VideoData::decode_length_delimited(&mut Cursor::new(decrypted_plaintext)).unwrap();

                                decode_data_sender.send(decoded.data.clone()).unwrap();

                                // if let Some(video_data) = &value.payload {
                                //     decode_data_sender.send(video_data.data.clone()).unwrap();
                                // }
                            }
                        }

                        // Update tracker for new matrix
                        *tracker_guard = Some(MatrixTracker::new(&packet));
                    }
                } else {
                    *tracker_guard = Some(MatrixTracker::new(&packet));
                }

                if let Some(ref mut tracker) = *tracker_guard {
                    tracker.add_packet(packet);
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
