use crate::proto::{IrisPacket, TransmitionPacket, ItemType};
use crossbeam_channel::{Receiver, Sender};
use log::{error, info};
use prost::Message;
use grid::Grid as Matrix;
use reed_solomon_16::{ReedSolomonDecoder, ReedSolomonEncoder};

use std::time::Instant;

pub const ROW: usize = 6;
pub const COL: usize = 6;
pub const ROW_FEC: usize = 3;
pub const COL_FEC: usize = 3;

const SHARD_SIZE: usize = 256;
const MTU_SIZE: usize = 1500;

const PACKET_PAYLOAD: usize = MTU_SIZE / SHARD_SIZE;

// This is running inside a separate task
pub fn start(rx: Receiver<Matrix<Vec<u8>>>, tx: Sender<Vec<u8>>) {
    let start_time = Instant::now();
    // Create transmission matrix

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

    let mut matrix_counter = 0;

    while let Ok(matrix) = rx.recv() {
        let mut transmission_matrix: Matrix<TransmitionPacket> = Matrix::new(0, 0);

        let mut matrix_index = 0;

        for rows in matrix.iter_rows() {
            let mut row: Vec<TransmitionPacket> = Vec::new();

            for (item_index, item) in rows.enumerate() {
                let packet = TransmitionPacket {
                    matrix: matrix_counter,
                    index: matrix_index as i32,
                    rows: (ROW + COL_FEC) as i32,
                    cols: (COL + ROW_FEC) as i32,
                    item_type: vec![
                        ItemType {
                            code: 2, // its data packet
                            index: item_index as i32 // index of the shard in the row
                        }],
                    payload: item.to_vec(), // copy data to TransmitionPacket
                };

                // Serialize Data
                let mut serialized_data = Vec::new();
                Message::encode(&packet, &mut serialized_data).unwrap_or_else(|error| {
                    eprintln!("Error: {}", error);
                });

                // Pad the serialized data to SHARD_SIZE len
                serialized_data.resize(SHARD_SIZE, 0);

                row.push(packet);

                row_encoder.add_original_shard(&serialized_data).unwrap();
                matrix_index += 1;
            }

            // Create FEC Shards for the ROW
            let result = row_encoder.encode().unwrap();

            for (item_index, item) in result.recovery_iter().enumerate() {
                let packet = TransmitionPacket {
                    matrix: matrix_counter,
                    index: matrix_index,
                    rows: (ROW + COL_FEC) as i32,
                    cols: (COL + ROW_FEC) as i32,
                    item_type: vec![
                        ItemType {
                            code: 0, // its row FEC packet
                            index: item_index as i32 // index of the shard in the row
                        }],
                    payload: item.to_vec(), // copy data to TransmitionPacket
                };

                // Serialize Data
                let mut serialized_data = Vec::new();
                Message::encode(&packet, &mut serialized_data).unwrap_or_else(|error| {
                    eprintln!("Error: {}", error);
                });

                // Pad the serialized data to SHARD_SIZE len
                serialized_data.resize(SHARD_SIZE, 0);

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
                item_type: vec![ItemType { code: 0, index: 0 }; 0],
                payload: vec![0; 0]
            }; COL + ROW_FEC]);
        }

        let column_data: Vec<Vec<TransmitionPacket>> = transmission_matrix.iter_cols().map(|col| col.cloned().collect()).collect();

        // Iterate over columns of transmission matrix to update the extra FEC info
        for (col_index, col) in column_data.iter().enumerate() {
            for item in col.iter().take(ROW) {
                // Serialize Data
                let mut serialized_data = Vec::new();
                Message::encode(item, &mut serialized_data).unwrap_or_else(|error| {
                    eprintln!("Error: {}", error);
                });

                // Pad the serialized data to SHARD_SIZE len
                serialized_data.resize(SHARD_SIZE, 0);

                // update col parity index
                col_encoder.add_original_shard(&serialized_data).unwrap();
            }

            // Create FEC Shards for the ROW
            let result = col_encoder.encode().unwrap();

            for (item_index, item) in result.recovery_iter().enumerate() {
                let packet = TransmitionPacket {
                    matrix: matrix_counter,
                    index: ((ROW + item_index) * (COL + ROW_FEC) + col_index) as i32,
                    rows: (ROW + COL_FEC) as i32,
                    cols: (COL + ROW_FEC) as i32,
                    item_type: vec![
                        ItemType {
                            code: 1, // its col FEC packet
                            index: item_index as i32 // index of the shard in the row
                        }],
                    payload: item.to_vec(), // copy data to TransmitionPacket
                };

                // Serialize Data
                let mut serialized_data = Vec::new();
                Message::encode(&packet, &mut serialized_data).unwrap_or_else(|error| {
                    eprintln!("Error: {}", error);
                });

                // Pad the serialized data to SHARD_SIZE len
                serialized_data.resize(SHARD_SIZE, 0);

                transmission_matrix[(ROW + item_index, col_index)] = packet;
            }
        }

        matrix_counter += 1;

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
                let mut iris_serialized_data = Vec::new();
                Message::encode(&iris_packet, &mut iris_serialized_data).unwrap_or_else(|error| {
                    eprintln!("Error: {}", error);
                });

                // Send the buffer
                tx.send(iris_serialized_data).unwrap_or_else(|e| {
                    error!("error sending data {:?}", e);
                });

                group.clear();
            }
        }

        // Send any remaining packets
        if !group.is_empty() {

            let iris_packet = IrisPacket {
                payload: group.clone(),
            };

            // Serialize IrisPacket
            let mut iris_serialized_data = Vec::new();
            Message::encode(&iris_packet, &mut iris_serialized_data).unwrap_or_else(|error| {
                eprintln!("Error: {}", error);
            });

            // Send the buffer
            tx.send(iris_serialized_data).unwrap_or_else(|e| {
                error!("error sending data {:?}", e);
            });
        }
    }
    info!("Transmission Matrix Index = {:?}", matrix_counter);

    let end_time = Instant::now();
    let duration = end_time.duration_since(start_time);

    info!("Time taken: {:?}", duration);
}
