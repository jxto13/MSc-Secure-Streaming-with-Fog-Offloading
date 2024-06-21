use crate::proto::{IrisPacket, TransmitionPacket};

use crossbeam_channel::{Receiver, Sender};
use rand::Rng;
use reed_solomon_16::ReedSolomonEncoder;
use grid::Grid as Matrix;
use prost::Message;

use std::time::Instant;

use log::{error, info};

pub const ROW: usize = 6;
pub const COL: usize = 6;
pub const ROW_FEC: usize = 3;
pub const COL_FEC: usize = 3;

const SHARD_SIZE: usize = 256;
const MTU_SIZE: usize = 1500;

const PACKET_PAYLOAD: usize = MTU_SIZE / SHARD_SIZE;

// Define the packet loss percentage
const PACKET_LOSS_PERCENTAGE: f64 = 0.0; // 2% packet loss

// This is running inside a separate task
pub fn start(rx: Receiver<Matrix<Vec<u8>>>, tx: Sender<Vec<u8>>) {
    // Create counters
    let start_time = Instant::now();
    let mut matrix_counter = 0;
    
    // Create encoders objects
    let mut row_encoder = ReedSolomonEncoder::new(
        COL, // total number of col, which gives the length of row
        ROW_FEC, // total number of row recovery shards
        SHARD_SIZE, // shard size in bytes
    ).unwrap();
    
    let mut col_encoder = ReedSolomonEncoder::new(
        ROW, // total number of col, which gives the length of row
        COL_FEC, // total number of row recovery shards
        SHARD_SIZE, // shard size in bytes
    ).unwrap();
    
    // Create transmission matrix
    while let Ok(matrix) = rx.recv() {
        let mut transmission_matrix: Matrix<TransmitionPacket> = Matrix::new(0, 0);

        let mut matrix_index = 0;

        for rows in matrix.iter_rows() {
            let mut row: Vec<TransmitionPacket> = Vec::new();

            for item in rows {
                let packet = TransmitionPacket {
                    matrix: matrix_counter,
                    index: matrix_index as i32,
                    rows: ROW as i32,
                    cols: COL as i32,
                    rows_fec: ROW_FEC as i32,
                    cols_fec: COL_FEC as i32,
                    fec: false,
                    payload: item.clone(), // copy data to TransmitionPacket, this will be VideoData encrypted
                };

                row.push(packet);

                row_encoder.add_original_shard(&item).unwrap();
                matrix_index += 1;
            }

            // Create FEC Shards for the ROW
            let result = row_encoder.encode().unwrap();

            for item in result.recovery_iter() {
                let packet = TransmitionPacket {
                    matrix: matrix_counter,
                    index: matrix_index,
                    rows: ROW as i32,
                    cols: COL as i32,
                    rows_fec: ROW_FEC as i32,
                    cols_fec: COL_FEC as i32,
                    fec: true,
                    payload: item.to_vec(), // copy data to TransmitionPacket
                };

                row.push(packet);

                matrix_index += 1;
            }
            transmission_matrix.push_row(row);
        }

        // Add COL_FEC as empty rows
        for _ in 0..COL_FEC {
            transmission_matrix.push_row(vec![TransmitionPacket {
                matrix: 0,
                index: 0,
                rows: 0,
                cols: 0,
                rows_fec: 0,
                cols_fec: 0,
                fec: false,
                payload: vec![0; 0]
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
                let packet = TransmitionPacket {
                    matrix: matrix_counter,
                    index: ((ROW + item_index) * (COL + ROW_FEC) + col_index) as i32,
                    rows: ROW as i32,
                    cols: COL as i32,
                    rows_fec: ROW_FEC as i32,
                    cols_fec: COL_FEC as i32,
                    fec: true,
                    payload: item.to_vec(), // copy data to TransmitionPacket
                };

                transmission_matrix[(ROW + item_index, col_index)] = packet;
            }
        }

        matrix_counter += 1;

        // Group the TransmitionPacket to create a IrisPacket with a number of bytes close to MTU
        let mut group = Vec::new();
        for item in transmission_matrix.iter() {

            // Serialize Data
            let mut serialized_data = Vec::new();
            Message::encode(item, &mut serialized_data).unwrap_or_else(|error| {
                eprintln!("Error: {}", error);
            });

            group.push(item.clone());

            if group.len() == PACKET_PAYLOAD {
                let iris_packet = IrisPacket {
                    payload: group.clone(),
                };

                // Serialize IrisPacket
                let iris_serialized_data = iris_packet.encode_to_vec();
                
                // Simulate packet loss
                if rand::thread_rng().gen::<f64>() * 100.0 >= PACKET_LOSS_PERCENTAGE {
                    // Send the buffer
                    tx.send(iris_serialized_data).unwrap_or_else(|e| {
                        error!("error sending data {:?}", e);
                    });
                }

                group.clear();
            }
        }

        // Send any remaining packets
        if !group.is_empty() {

            let iris_packet = IrisPacket {
                payload: group.clone(),
            };

            // Serialize IrisPacket
            let iris_serialized_data = iris_packet.encode_to_vec();
            
            // Simulate packet loss
            if rand::thread_rng().gen::<f64>() * 100.0 >= PACKET_LOSS_PERCENTAGE {
                // Send the buffer
                tx.send(iris_serialized_data).unwrap_or_else(|e| {
                    error!("error sending data {:?}", e);
                });
            }
        }
    }
    info!("Last Transmission Matrix Index = {:?}", matrix_counter);

    let end_time = Instant::now();
    let duration = end_time.duration_since(start_time);

    info!("Time taken to encode and send the data to multicast sender: {:?}", duration);
}
