use crate::crypto::key_manager::{self, KeyManager};


use crate::{CONFIG, MANAGER};

use crossbeam_channel::{Receiver, Sender};
use iris::crypto_utils::encrypt;
use iris::streaming_server;
use iris::streaming_server::proto::{SessionKey, TransmitionPacket, VideoData};
use openssl::ecdsa::EcdsaSig;
use openssl::symm::Cipher;
use reed_solomon_16::ReedSolomonEncoder;
use grid::Grid as Matrix;
use prost::Message;

use std::io::Cursor;
use std::time::Instant;
use log::{error, info};

use openssl::sha::sha256;

// This is running inside a separate task
pub fn start(rx: Receiver<Matrix<Vec<u8>>>, tx: Sender<Vec<u8>>) {
    let config = &CONFIG.get().unwrap().multicast;
    
    let ROW: usize = config.row;
    let COL: usize = config.col;
    let ROW_FEC: usize = config.row_fec;
    let COL_FEC: usize = config.col_fec;
    
    let SHARD_SIZE: usize = config.shard_size;
    let MTU_SIZE: usize = config.mtu_size;
    
    assert!(SHARD_SIZE < MTU_SIZE);

    // Create counters
    let start_time = Instant::now();
    let mut matrix_counter = 0;
    let mut epoch = 0;
    
    // Create encoders objects
    let mut row_encoder = match ReedSolomonEncoder::new(
        COL,       // total number of col, which gives the length of row
        ROW_FEC,   // total number of row recovery shards
        SHARD_SIZE // shard size in bytes
    ) {
        Ok(encoder) => Some(encoder),
        Err(_) => {
            None
        }
    };
    
    let mut col_encoder = match ReedSolomonEncoder::new(
        ROW,       // total number of col, which gives the length of row
        COL_FEC,   // total number of row recovery shards
        SHARD_SIZE // shard size in bytes
    ) {
        Ok(encoder) => Some(encoder),
        Err(_) => {
            None
        }
    };
    
    // Create transmission matrix
    while let Ok(matrix) = rx.recv() {
        let mut transmission_matrix: Matrix<TransmitionPacket> = Matrix::new(0, 0);

        let mut matrix_index = 0;
        let manager = MANAGER.get().unwrap();
        let mut binding = manager.lock().unwrap();

        // If epoch is equal to config, rotate keys and reset counter
        if epoch == config.epoch {
            info!("Key rotation performed after {} epochs", config.epoch);
            binding.rotate_key();
        
            epoch = 0;
        }
        
        // Get keys outside the loop
        let (ec_key, current_key, next_key) = {
            let ec_key = binding.get_ec_key().clone();
            let current_key = binding.get_current_key().clone();
            let next_key = binding.get_next_key().clone();
            (ec_key, current_key, next_key)
        };

        for rows in matrix.iter_rows() {
            let mut row: Vec<TransmitionPacket> = Vec::new();

            for item in rows {
                let mut video_data = VideoData {
                    data: item.clone(),
                    signature: vec![0,0],
                    next_key: Some(next_key.clone()),
                };

                // Create hash of message and sign
                let message_hash = sha256(&video_data.encode_to_vec());
                let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap().to_der().unwrap();

                video_data.signature = signature;

                assert!(video_data.encode_to_vec().len() < SHARD_SIZE);
                
                let mut packet = TransmitionPacket {
                    matrix: matrix_counter,
                    index: matrix_index as i32,
                    rows: ROW as i32,
                    cols: COL as i32,
                    rows_fec: ROW_FEC as i32,
                    cols_fec: COL_FEC as i32,
                    payload: encrypt(&video_data.encode_length_delimited_to_vec(), &current_key.key, &current_key.iv, SHARD_SIZE),
                    packet_signature: vec![0,255],
                };
                let message_hash = sha256(&packet.encode_to_vec());
                let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap().to_der().unwrap();
                
                packet.packet_signature = signature;
                
                row.push(packet.clone());
                
                // Check if we have a row_encoder
                if let Some(ref mut row_encoder) = row_encoder {
                    row_encoder.add_original_shard(&packet.payload).unwrap();                    
                }

                matrix_index += 1;
            }


            if let Some(ref mut row_encoder) = row_encoder {

                // Create FEC Shards for the ROW
                let result = row_encoder.encode().unwrap();

                for item in result.recovery_iter() {

                    let mut packet = TransmitionPacket {
                        matrix: matrix_counter,
                        index: matrix_index,
                        rows: ROW as i32,
                        cols: COL as i32,
                        rows_fec: ROW_FEC as i32,
                        cols_fec: COL_FEC as i32,
                        payload: item.to_vec(),
                        packet_signature: vec![0,255],
                    };

                    let message_hash = sha256(&packet.encode_to_vec());
                    let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap().to_der().unwrap();

                    packet.packet_signature = signature;

                    row.push(packet);

                    matrix_index += 1;
                }
            }
            transmission_matrix.push_row(row);
        }

        // Check if we have a col_encoder
        if let Some(ref mut col_encoder) = col_encoder {
           
            // Add COL_FEC as empty rows
            for _ in 0..COL_FEC {
                transmission_matrix.push_row(vec![TransmitionPacket {
                    matrix: 0,
                    index: 0,
                    rows: 0,
                    cols: 0,
                    rows_fec: 0,
                    cols_fec: 0,
                    payload: vec![0,0],
                    packet_signature: vec![0,255]
                }; COL + ROW_FEC]);
            }

            let column_data: Vec<Vec<TransmitionPacket>> = transmission_matrix.iter_cols().map(|col| col.cloned().collect()).collect();

            // Iterate over columns of transmission matrix to update the extra FEC info
            for (col_index, col) in column_data.iter().enumerate() {
                for item in col.iter().take(ROW) {
                    // update col parity index
                    col_encoder.add_original_shard(&item.payload).unwrap();
                }

                // Create FEC Shards for the ROW
                let result = col_encoder.encode().unwrap();

                for (item_index, item) in result.recovery_iter().enumerate() {

                    let mut packet = TransmitionPacket {
                        matrix: matrix_counter,
                        index: ((ROW + item_index) * (COL + ROW_FEC) + col_index) as i32,
                        rows: ROW as i32,
                        cols: COL as i32,
                        rows_fec: ROW_FEC as i32,
                        cols_fec: COL_FEC as i32,
                        payload: item.to_vec(),
                        packet_signature: vec![0,255],
                    };

                    let message_hash = sha256(&packet.encode_to_vec());
                    let signature = EcdsaSig::sign(&message_hash, &ec_key).unwrap().to_der().unwrap();

                    packet.packet_signature = signature;

                    transmission_matrix[(ROW + item_index, col_index)] = packet;
                }
            }
        }

        matrix_counter += 1;

        // Loop the matrix col by col
        for col in transmission_matrix.iter_cols() {

            for item in col {
                // Serialize IrisPacket
                let iris_serialized_data = item.encode_to_vec();
                // Send the buffer
                tx.send(iris_serialized_data).unwrap_or_else(|e| {
                    panic!("error sending data {:?}", e);
                });
            }
        }

        // Increment epoch
        epoch += 1;

        
    }

    let end_time = Instant::now();
    let duration = end_time.duration_since(start_time);

    info!("Time taken to encode and send the data to multicast sender: {:?}", duration);
}
