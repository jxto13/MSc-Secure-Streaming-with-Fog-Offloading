use crossbeam_channel::{tick, Receiver, Sender, TryRecvError};
use iris_shared::padding;
use log::{error, info};
use openssl::ec::{EcKey, EcKeyRef};
use openssl::pkey::{Private, Public};
use openssl::sha::sha256;
use openssl::x509::X509;
use prost::Message;

use std::collections::VecDeque;
use std::error::Error;
use std::io::Cursor;
use std::thread;
use std::time::{Duration, Instant};

use grid::Grid as Matrix;

use openssl::ecdsa::EcdsaSig;

use iris_shared::crypto_utils::decrypt;

use crate::proto::{MatrixReportBitmap, PacketWrapper, RetransmissionPacket, SessionKey, TransmissionPacket, VideoData};
use crate::receiver::ReceiverType;
use crate::proto::packet_wrapper::PacketType;
use crate::SESSION;

use super::proto::ClientReport;

#[derive(Debug)]
pub enum TickType {
    Report,
    DecodeToApp,
}

#[derive(Default,Debug)]
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
            bitmap: vec![0; ((packet.rows + packet.cols_fec) * (packet.cols + packet.rows_fec)) as usize], // Initialize the bitmap with 0s
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

        if self.matrix_packets[(row, col)].is_some(){
            error!("Packet already exists in the matrix at index: {} - {}", row, col);
            return;
        }

        // Add the packet to the matrix
        self.matrix_packets[(row, col)] = Some(packet.clone());
        
        // Update the bitmap if this is a data packet
        if (packet.index as usize) < self.bitmap.len() {
            self.bitmap[packet.index as usize] = 1;
        }


    }

    /// Reconstruct the matrix using the Reed-Solomon algorithm.
    fn reconstruct_matrix(&mut self, ec_client_key: &EcKey<Private>) -> Result<(), Box<dyn Error>> {
        // This let was do to a refactor, can be improved by replacing self in all places of this function
        let tracker = self;

        let mut updates= true;

        // Logic to reconstruct the matrix
        while updates {
            let mut new_packets_added = false;


            // Reconstruct rows
            for row_index in 0..tracker.matrix_packets.rows() {
    
                let current_row = tracker.matrix_packets.iter_row(row_index);
    
                let original_shards: Vec<(usize, &Vec<u8>)> = current_row
                    .clone()// Clone the iterator to avoid consuming it
                    .take(tracker.cols as usize)
                    .enumerate()
                    .filter_map(|(i, opt_packet)| opt_packet.as_ref().map(|packet| (i, &packet.payload)))
                    .collect();
    
                let recovery_shards: Vec<(usize, &Vec<u8>)> = current_row
                    .skip(tracker.cols as usize)
                    .enumerate()
                    .filter_map(|(i, opt_packet)| opt_packet.as_ref().map(|packet| (i, &packet.payload)))
                    .collect();
                
                // Reed-Solomon decode the row
                if let Ok(recovered) = reed_solomon_16::decode(
                    tracker.cols as usize,     // total number of original shards
                    tracker.rows_fec as usize, // total number of recovery shards
                    original_shards,
                    recovery_shards,
                ) {
                    for (position, data) in recovered {
    
                        let index = (tracker.cols + tracker.rows_fec) * row_index as u32 + position as u32;
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
                        log::debug!("Reconstructed packet: {:?}", packet.index);
                        new_packets_added = true;
                    }
                }
            }


            // Reconstruct collumn
            for col_index in 0..tracker.matrix_packets.cols() {

                let current_col = tracker.matrix_packets.iter_col(col_index);

                let original_shards: Vec<(usize, &Vec<u8>)> = current_col
                    .clone()// Clone the iterator to avoid consuming it
                    .take(tracker.rows as usize)
                    .enumerate()
                    .filter_map(|(i, opt_packet)| opt_packet.as_ref().map(|packet| (i, &packet.payload)))
                    .collect();

                let recovery_shards: Vec<(usize, &Vec<u8>)> = current_col
                    .skip(tracker.rows as usize)
                    .enumerate()
                    .filter_map(|(i, opt_packet)| opt_packet.as_ref().map(|packet| (i, &packet.payload)))
                    .collect();
                
                // Reed-Solomon decode the column
                if let Ok(recovered) = reed_solomon_16::decode(
                    tracker.rows as usize,     // total number of original shards
                    tracker.cols_fec as usize, // total number of recovery shards
                    original_shards,
                    recovery_shards,
                ) {
                    for (position, data) in recovered {

                        
                        let index = position as u32 * (tracker.cols + tracker.rows_fec) + col_index as u32;
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
                
                        // Sign the message hash with the client private key
                        let signature = EcdsaSig::sign(&message_hash, ec_client_key).unwrap().to_der().unwrap();
                
                        // Update the packet signature
                        packet.packet_signature = signature;
                        
                        tracker.add_packet(&packet);
                        log::debug!("Reconstructed packet: {:?}", packet.index);

                        new_packets_added = true;

                    }
                }

            }

            updates = new_packets_added;
        }
        
        Ok(())
    }

    /// Checks if matrix is complete or not
    fn is_complete(&self) -> bool {
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

/// Process a packet and check if it belongs to a new matrix or an old matrix in the buffer.
fn process_packet(buffer: &mut VecDeque<MatrixTracker>, packet: TransmissionPacket, current_key: &mut SessionKey, next_key: &mut Option<SessionKey>) {
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
            tracker.reconstruct_matrix(&session_config.client_ec_key.as_ref().unwrap()).unwrap();

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
    for tracker in buffer.iter_mut(){
        if packet.matrix == tracker.matrix_index {
            // log::debug!("Matrix index: {} - Packet index: {}", tracker.matrix_index, packet.index);
            // log::debug!("tracker before adding packet: {:?}", tracker.bitmap);
            tracker.add_packet(&packet);
            // log::debug!("tracker after adding packet: {:?}", tracker.bitmap);
        }
    }    
}

/// Check if the key rotation is necessary and rotate the key if necessary. Only called when the tracker is created.
fn check_key_rotation(matrix: &mut MatrixTracker, packet: &TransmissionPacket, current_key: &mut SessionKey, next_key: &mut Option<SessionKey>) {
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

                // Decrypt the matrix with the next_key, because the matrix is from the next epoch
                let decrypted_plaintext = decrypt(&deppaded_payload, &next_key_value.key, &next_key_value.iv).unwrap();

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
            },
            None => {
                return;
            }
        };

        // Check if the new key is not None
        match new_key {
            Some(new_key_value) => {
                // Rotate the key
                rotate_key(current_key, next_key, new_key_value);
            },
            None => {
                error!("New key is not set.");
                return;
            }
        }
    } 
}

/// Rotate the current key and next key.
fn rotate_key(current_key: &mut SessionKey, next_key: &mut Option<SessionKey>, new_key: SessionKey) {
    *current_key = next_key.take().unwrap();
    *next_key = Some(new_key);
}

pub fn start(
    data_receiver: Receiver<(ReceiverType, Vec<u8>)>,
    decode_data_sender: Sender<Vec<u8>>,
    client_report_sender: Sender<ClientReport>,
) {

    let mut buffer: VecDeque<MatrixTracker> = VecDeque::new();

    let session_config = SESSION.get().unwrap().read().unwrap();

    // Public Key from Server to verify packets signature
    let x509_public_key = X509::from_pem(&session_config.ec_server_certificate).unwrap().public_key().unwrap().ec_key().unwrap();

    // Current encryption key and Option for next encryption key
    let mut current_key_session = session_config.current_key.clone();
    let mut next_key_session : Option<SessionKey> = Some(session_config.next_key.clone());
    
    // Client EC Key
    let client_ec_key = if let Some(ec_key) = &session_config.client_ec_key {
        ec_key
    } else {
        error!("Client EC Key is not set.");
        // return Err("Client EC Key is not set.".into());
        panic!("Client EC Key is not set.");
    };

    let mut report_counter= 0;
    let client_id = sha256(&client_ec_key.public_key_to_der().unwrap()).to_vec();
    
    
    let rint = Duration::from_millis(200); // interval between retries
    let d = Duration::from_millis(1000);   // total retry window
    let max_rounds = d.as_millis() / rint.as_millis(); // 5
    
    let retry_tick = tick(rint);
    let mut attempt = 0;

    loop {
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
                            info!("Buffer is empty or has only one item, skipping report.");
                            continue;
                        }

                        // Create a new ClientReport
                        let mut report = ClientReport {
                            client_id: client_id.clone(),
                            counter: report_counter.clone(),
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
                        report.signature = EcdsaSig::sign(&sha256(&report.encode_to_vec()), &client_ec_key).unwrap().to_der().unwrap();

                        // Send the report
                        client_report_sender.send(report).unwrap();

                        // Increment the report counter
                        report_counter += 1;
                    
                    },
                    TickType::DecodeToApp => {
                        info!("Decoding to application layer...");
                        
                        // Check if the buffer has more than one item
                        if buffer.len() > 1 {
            
                            // Drain all elements except the last one
                            let drained : Vec<MatrixTracker> = buffer.drain(0..buffer.len() - 1).collect();
            
                            for mut item in drained {
                                
                                // Reconstruct the matrix
                                item.reconstruct_matrix(&client_ec_key).unwrap();

                                log::debug!("Missing packets: {:?} - {:?}", item.matrix_index, item.missing_indexes());

                                // Decode the matrix to the application layer
                                decode_to_app_layer(&mut item, &decode_data_sender, &x509_public_key).unwrap();
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
                        println!("Channel closed: {:?}", err);
                        break;
                    }
                };

                let wrapper: Vec<u8> = match source {
                    // This can be helpful to track metrics
                    ReceiverType::Multicast=>{
                        data
                    },
                    ReceiverType::Unicast=>{
                        data
                    },
                    ReceiverType::WifiAware => {
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
                        if let Some(packet) = retransmission.packet {
                            // Missing logic to decode linear combination with the other packets of matrix
                            // info!("RetransmissionPacket contained packet: {:?} - {:?}", packet.matrix, packet.index);
                            process_retransmission_packet(&mut buffer, packet);

                        } else {
                            eprintln!("RetransmissionPacket contained no packet.");
                        }
                    }
                    None => {
                        // no packet set — error or ignore
                    }
                }


            }
        }
    }
    // Main loop to receive all packets from receiving threads
    // loop {
        
    //     crossbeam_channel::select! {
    //         recv(timers_receiver) -> tick => {
    //             match tick {
    //                 Ok(TickType::Report) => {

    //                     // Check if the buffer has more than one item
    //                     if buffer.len() < 1 {
    //                         info!("Buffer is empty or has only one item, skipping report.");
    //                         continue;
    //                     }

    //                     // Create a new ClientReport
    //                     let mut report = ClientReport {
    //                         client_id: client_id.clone(),
    //                         counter: report_counter.clone(),
    //                         reports: Vec::with_capacity(buffer.len() - 1),
    //                         signature: vec![0, 255],
    //                     };

    //                     // Iterate over the buffer and collect reports
    //                     for item in buffer.iter().take(buffer.len() - 1) {
    //                         report.reports.push(MatrixReportBitmap {
    //                             matrix_index: item.matrix_index,
    //                             bitmap: item.bitmap.clone(),
    //                         });
    //                     }

    //                     // Sign the report
    //                     report.signature = EcdsaSig::sign(&sha256(&report.encode_to_vec()), &client_ec_key).unwrap().to_der().unwrap();

    //                     // Send the report
    //                     client_report_sender.send(report).unwrap();

    //                     // Increment the report counter
    //                     report_counter += 1;
                    
    //                 },
    //                 Ok(TickType::DecodeToApp) => {
                        
    //                     // Check if the buffer has more than one item
    //                     if buffer.len() > 1 {
            
    //                         // Drain all elements except the last one
    //                         let drained : Vec<MatrixTracker> = buffer.drain(0..buffer.len() - 1).collect();
            
    //                         for mut item in drained {
                                
    //                             // Reconstruct the matrix
    //                             item.reconstruct_matrix(&client_ec_key).unwrap();

    //                             log::debug!("Missing packets: {:?} - {:?}", item.matrix_index, item.missing_indexes());

    //                             // Decode the matrix to the application layer
    //                             decode_to_app_layer(&mut item, &decode_data_sender, &x509_public_key).unwrap();
    //                         }
            
    //                     }

    //                 },
    //                 Err(_) => {
    //                     error!("Timers channel disconnected.");
    //                 }
    //             }
    //         },
    //         recv(multicast_receiver) -> payload => {
    //             match payload {
    //                 Ok(payload_bytes) => {
    //                     // Attempt to decode as a TransmissionPacket
    //                     if let Ok(mut transmission) = TransmissionPacket::decode(&*payload_bytes) {
    //                         // Validate the packet signature, if invalid, skip the packet
    //                         if validate_packet(&mut transmission, &x509_public_key) {
    //                             // Process the packet if valid
    //                             process_packet(&mut buffer, transmission, &mut current_key_session, &mut next_key_session);
    //                         } else {
    //                             // Invalid packet, skip
    //                             continue;
    //                         }
    //                     }

    //                     // Attempt to decode as a RetransmissionPacket
    //                     if let Ok(retransmission) = RetransmissionPacket::decode(&*payload_bytes) {
    //                         if let Some(packet) = retransmission.packet {

    //                             // Missing logic to decode linear combination with the other packets of matrix
    //                             // info!("RetransmissionPacket contained packet: {:?} - {:?}", packet.matrix, packet.index);
    //                             process_retransmission_packet(&mut buffer, packet);

    //                         } else {
    //                             eprintln!("RetransmissionPacket contained no packet.");
    //                         }
    //                     }
    //                 },
    //                 Err(recv_error) => {
    //                     // No message available, continue looping
    //                     error!("Failed to receive multicast packet: {}", recv_error);
    //                 }
    //             }
    //         },

    //         recv(unicast_receiver) -> payload => {
    //             match payload {
    //                 Ok(payload_bytes) => {
    //                     // Attempt to decode as a RetransmissionPacket
    //                     if let Ok(retransmission) = RetransmissionPacket::decode(&*payload_bytes) {
    //                         if let Some(mut packet) = retransmission.packet {
    //                             // Validate the packet signature, if invalid, skip the packet
    //                             if validate_packet(&mut packet, &x509_public_key) {
    //                                 // Process the packet if valid
    //                                 process_retransmission_packet(&mut buffer, packet);
    //                             }
    //                         } else {
    //                             error!("RetransmissionPacket contained no packet.");
    //                         }
    //                     }
    //                 },
    //                 Err(recv_error) => {
    //                     // No message available, continue looping
    //                     error!("Failed to receive unicast packet: {}", recv_error);
    //                 }
    //             }
    //         },
            

    //     }
    // }
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
            // remove padding from payload
            let deppaded_payload = padding::depad(&value.payload).unwrap();

            // Decrypt the payload
            let decrypted_plaintext = decrypt(&deppaded_payload, &session_key.key, &session_key.iv).unwrap();
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
                decode_data_sender.send(decoded.data).unwrap();
            }
        }
    }

    Ok(())
}
