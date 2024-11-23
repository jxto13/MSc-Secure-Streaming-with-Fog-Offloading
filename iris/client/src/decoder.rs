use chrono::{DateTime, Utc};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use log::{error, info};
use openssl::ec::{EcKey, EcKeyRef};
use openssl::pkey::{Private, Public};
use openssl::sha::sha256;
use openssl::x509::X509;
use prost::Message;

use std::collections::VecDeque;
use std::error::Error;
use std::io::Cursor;
use std::time::Instant;

use grid::Grid as Matrix;

use openssl::ecdsa::EcdsaSig;

use iris_shared::crypto_utils::decrypt;

use crate::proto::{SessionKey, TransmissionPacket, VideoData, MatrixReportBitmap, RetransmissionPacket};
use crate::TIME;

use super::proto::ClientReport;
use super::utils::SessionConfig;

#[derive(Default)]
struct MatrixTracker {
    matrix_index: i32,
    cols: i32,
    rows: i32,
    cols_fec: i32,
    rows_fec: i32,
    matrix_packets: Matrix<Option<TransmissionPacket>>,
    session_key: Option<SessionKey>, 
    bitmap: Vec<u8>, // Bitmap to track the presence of packets
    key_rotated: bool, // New field to track if the key has been rotated for this matrix
}

impl MatrixTracker {
    fn new(packet: &TransmissionPacket) -> Self {
         // Initialize the MatrixTracker with the packet's matrix details.
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
            bitmap: vec![0; ((packet.rows + packet.cols_fec) * (packet.cols + packet.rows_fec)) as usize], // Initialize the bitmap with 0s
            key_rotated: false, // Initialize key rotation status
        };

        // Add the initial packet to the tracker.
        tracker.add_packet(packet);

        // Return the tracker.
        tracker
    }

    // Add packet to Tracker
    fn add_packet(&mut self, packet: &TransmissionPacket) {
        let index = packet.index as usize;
        let row = index / (self.cols + self.rows_fec) as usize;
        let col = index % (self.cols + self.rows_fec) as usize;
        self.matrix_packets[(row, col)] = Some(packet.clone());

        // Update the bitmap if this is a data packet
        {
            let data_index = row * self.cols as usize + col;
            self.bitmap[data_index] = 1; // Set the corresponding position in the bitmap to 1
        }

    }

    fn reconstruct_matrix(&mut self, ec_client_key: &EcKey<Private>) -> Result<(), Box<dyn Error>> {
        // This let was do to a refactor, can be improved by replacing self in all places of this function
        let tracker = self;
        
        // Temporarily take `matrix_packets` out
        let mut matrix_packets = std::mem::take(&mut tracker.matrix_packets);
        
        // Logic to reconstruct the matrix
        // Reconstruct rows
        for row_index in 0..tracker.rows as usize {
            
            // Get mut row by index 
            for row in matrix_packets.iter_row_mut(row_index){

                // Collect the first M (original) and remaining N-M (recovery) elements in a single iteration
                let (original_shards, recovery_shards): (Vec<_>, Vec<_>) = row.iter_mut()
                .enumerate()
                .partition(|(index, _)| *index < tracker.cols as usize);  // Split based on the index

                // Convert original_shards and recovery_shards into the correct format
                let original_shards: Vec<(usize, &mut Vec<u8>)> = original_shards
                    .into_iter()
                    .map(|(index, transmition_packet)| (index, &mut transmition_packet.payload))
                    .collect();

                let recovery_shards: Vec<(usize, &mut Vec<u8>)> = recovery_shards
                    .into_iter()
                    .map(|(index, transmition_packet)| (index - tracker.cols as usize, &mut transmition_packet.payload))  // Adjust the index for recovery shards
                    .collect();

                // Reed-Solomon decode the row
                if let Ok(recovered) = reed_solomon_16::decode(
                    tracker.cols as usize,     // total number of original shards
                    tracker.rows_fec as usize, // total number of recovery shards
                    original_shards,
                    recovery_shards,
                ) {
                    for (position, data) in recovered {

                        let index = (tracker.cols + tracker.rows_fec) * row_index as i32 + position as i32;

                        let mut packet = TransmissionPacket {
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
                        
                        tracker.add_packet(&packet);                    
                    }
                }
            }
        }

        // Reconstruct columns
        for col_index in 0..tracker.cols as usize {

            for col in matrix_packets.iter_col_mut(col_index){

                // Collect the first M (original) and remaining N-M (recovery) elements in a single iteration
                let (original_shards, recovery_shards): (Vec<_>, Vec<_>) = col.iter_mut()
                    .enumerate()
                    .partition(|(index, _)| *index < tracker.rows as usize);  // Split based on the index
                
                // Convert original_shards and recovery_shards into the correct format
                let original_shards: Vec<(usize, &mut Vec<u8>)> = original_shards
                    .into_iter()
                    .map(|(index, transmition_packet)| (index, &mut transmition_packet.payload))
                    .collect();

                let recovery_shards: Vec<(usize, &mut Vec<u8>)> = recovery_shards
                    .into_iter()
                    .map(|(index, transmition_packet)| (index - tracker.rows as usize, &mut transmition_packet.payload))  // Adjust the index for recovery shards
                    .collect();

                // Reed-Solomon decode the column
                if let Ok(recovered) = reed_solomon_16::decode(
                    tracker.rows as usize,     // total number of original shards
                    tracker.cols_fec as usize, // total number of recovery shards
                    original_shards,
                    recovery_shards,
                ) {
                    for (position, data) in recovered {
                        
                        let index = (tracker.rows + tracker.cols_fec) * col_index as i32 + position as i32;
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
                        let encoded_packet = packet.encode_to_vec();  // Replace with the actual encoding method you're using
                        let message_hash = sha256(&encoded_packet);
                
                        // Sign the message hash
                        let signature = EcdsaSig::sign(&message_hash, ec_client_key).unwrap().to_der().unwrap();
                
                        // Update the packet signature
                        packet.packet_signature = signature;
                        
                        tracker.add_packet(&packet);
                    }
                }
            }
        }

        // Restore `matrix_packets`
        tracker.matrix_packets = matrix_packets;

        
        Ok(())
    }

    // Checks if matrix is complete or not
    fn is_complete(&self) -> bool {
        self.bitmap.iter().map(|&bit| bit == 1).all(|x| x)
    }

    // Returns the missing index (the index are in row major) as vector
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

pub fn start(
    multicast_receiver: Receiver<Vec<u8>>,
    unicast_receiver: Receiver<Vec<u8>>,
    decode_data_sender: Sender<Vec<u8>>,
    client_report_sender: Sender<ClientReport>,
    session_config: SessionConfig,
) -> Result<(), Box<dyn Error>> {
    let mut buffer: VecDeque<MatrixTracker> = VecDeque::new();

    // Public Key from Server to verify packets signature
    let x509_public_key = X509::from_pem(&session_config.ec_server_certificate)?.public_key()?.ec_key().unwrap();

    // Current encryption key and Option for next encryption key
    let mut current_key_session = session_config.current_key;
    let mut next_key_session : Option<SessionKey> = Some(session_config.next_key);

    let mut server_time = None;

    {
        let current_server_time = TIME.get().unwrap().lock().unwrap();
        server_time = Some(*current_server_time);
    }
    
    // Simulated current server time (this will be periodically updated)
    let mut flush_buffer_timer = server_time.unwrap();
    let mut client_report_timer = server_time.unwrap();

    let mut process_packet = |buffer: &mut VecDeque<MatrixTracker>,mut packet: TransmissionPacket| {

        // Get the signature from the packet and put the packet_signature field with 0's to calculate hash using local computation
        let signature = EcdsaSig::from_der(&packet.packet_signature).unwrap();
        packet.packet_signature = vec![0, 255];
        let message_hash = sha256(&packet.encode_to_vec());

        let is_valid = signature.verify(&message_hash, &x509_public_key).unwrap();

        // If packet signature is not valid, drop and continue
        if !is_valid {
            return;
        }

        // Check if next_key_session is null
        if next_key_session == None {
            let decrypted_plaintext = decrypt(&packet.payload, &current_key_session.key, &current_key_session.iv);
            if !decrypted_plaintext.len() != 0 {
                let decoded = match VideoData::decode_length_delimited(&mut Cursor::new(decrypted_plaintext)){
                    Ok(value) => value,
                    Err(err) => {
                        error!("{}", err);
                        return;
                    },
                };
                next_key_session = decoded.next_key;
            }
        }

        // Check if the packet belongs to an existing matrix in the buffer
        if let Some(tracker) = buffer.iter_mut().rev().find(|tracker| tracker.matrix_index == packet.matrix) {
            // Rotate the session key only if it hasn't been rotated for this matrix tracker
            if packet.matrix % session_config.epoch == 0 && !tracker.key_rotated {
                if let Some(new_session_key) = next_key_session.take() {
                    current_key_session = new_session_key;
                    tracker.key_rotated = true; // Mark rotation as done for this matrix
                }
            }
            
            tracker.add_packet(&packet);
            
        } else { // Else is a packet from the next matrix
            // Create a new MatrixTracker
            let mut new_tracker = MatrixTracker::new(&packet);

            // Rotate the session key if needed for this new matrix
            if packet.matrix % session_config.epoch == 0 && !new_tracker.key_rotated {
                if let Some(new_session_key) = next_key_session.take() {
                    current_key_session = new_session_key;
                    new_tracker.key_rotated = true; // Mark rotation as done for this matrix
                }
            }

            new_tracker.session_key = Some(current_key_session.clone());

            // Add it to the buffer
            buffer.push_back(new_tracker);
        }
    };

    // Main loop to receive all packets from receiving threads
    loop {

        // Try to receive and decode multicast packets  
        match multicast_receiver.try_recv() {
            Ok(payload_bytes) => {

                // Attempt to decode as a TransmissionPacket
                if let Ok(transmission) = TransmissionPacket::decode(&*payload_bytes) {
                    // println!("TransmissionPacket data: {:?}", transmission);
                    process_packet(&mut buffer, transmission);
                }
                // Attempt to decode as a RetransmissionPacket
                if let Ok(retransmission) = RetransmissionPacket::decode(&*payload_bytes) {
                    // println!("RetransmissionPacket data: {:?}", retransmission);
                    if let Some(packet) = retransmission.packet {
                        // Missing logic to decode linear combination with the other packets of matrix
                        // process_packet(&mut buffer, packet);
                    } else {
                        eprintln!("RetransmissionPacket contained no packet.");
                    }
                }
               
            }
            Err(TryRecvError::Empty) => {
                // No message available, continue looping
            }
            Err(TryRecvError::Disconnected) => {
                // The channel is closed, continue loop
            }
        }

        // Try to receive and decode unicast packets  
        match unicast_receiver.try_recv() {
            Ok(payload_bytes) => {

                // Attempt to decode as a TransmissionPacket
                if let Ok(transmission) = TransmissionPacket::decode(&*payload_bytes) {
                    // println!("TransmissionPacket data: {:?}", transmission);
                    process_packet(&mut buffer, transmission);
                }
                // Attempt to decode as a RetransmissionPacket
                if let Ok(retransmission) = RetransmissionPacket::decode(&*payload_bytes) {
                    // println!("RetransmissionPacket data: {:?}", retransmission);
                    if let Some(packet) = retransmission.packet {
                        // Missing logic to decode linear combination with the other packets of matrix
                        // process_packet(&mut buffer, packet);
                    } else {
                        eprintln!("RetransmissionPacket contained no packet.");
                    }
                }
               
            }
            Err(TryRecvError::Empty) => {
                // No message available, continue looping
            }
            Err(TryRecvError::Disconnected) => {
                // The channel is closed, continue loop
            }
        }
        

        {
            
            // Check the time and send reports if 50ms have passed since the last one
            let now = Utc::now();

            if now - flush_buffer_timer >= chrono::Duration::milliseconds(500) {


                // Check size of buffer
                if buffer.len() > 1 {

                    // Drain all elements except the last one
                    let drained : Vec<MatrixTracker> = buffer.drain(0..buffer.len() - 1).collect();

                    for mut item in drained {
                        decode_to_app_layer(
                            &mut item,
                            &decode_data_sender,
                            &x509_public_key
                        ).unwrap();
                    }

                }

                // Reset timer
                flush_buffer_timer = now;
            }

            if now - client_report_timer >= chrono::Duration::milliseconds(200) {

                let mut bitmaps = Vec::new();

                // Loops all buffer and retrieves matrix_index and bitmaps of every tracker
                for item in buffer.iter() {
                    bitmaps.push(
                        MatrixReportBitmap {
                            matrix: item.matrix_index,
                            bitmap: item.bitmap.clone(),
                        }
                    );
                }
            
                let report = ClientReport {
                    reports: bitmaps,
                };

                client_report_sender.send(report).unwrap();
                
                // Reset timer
                client_report_timer = now;
            }
        }
    }
}

fn decode_to_app_layer(
    tracker: &mut MatrixTracker,
    decode_data_sender: &Sender<Vec<u8>>,
    x509_public_key: &EcKeyRef<Public>
) -> Result<(), Box<dyn Error>> {

    // Logic to remove FEC rows and columns
    for _ in 0..tracker.rows_fec {
        tracker.matrix_packets.pop_col();
    }
    
    for _ in 0..tracker.cols_fec {
        tracker.matrix_packets.pop_row();
    }

    // Ensure the session key is set
    let session_key = match tracker.session_key {
        Some(ref key) => key,
        None => {
            error!("Session key is not set for matrix {}", tracker.matrix_index);
            return Err("Session key is not set".into());
        }
    };
    
    // Decrypt and decode video data
    for item in tracker.matrix_packets.iter() {
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

    Ok(())
}

// fn try_decode(matrix_cell: &[u8]) -> Option<TransmissionPacket> {
    
//     if let Ok(transmition_packet) = TransmissionPacket::decode(matrix_cell) {
//         return Some(transmition_packet);
//     }

//     if let Ok(retransmition_packet) = RetransmissionPacket::decode(matrix_cell) {
//         // Missing logic to reconstruct the packet from the linear combination, the return value is the original packet reconstructed
//         return retransmition_packet.packet; // Unwrap the inner packet if needed
//     }

//     None
// }