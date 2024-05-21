use crate::proto::{DataPacket,IrisPacket};

use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read};

use std::sync::mpsc::Sender;
use std::thread::{sleep, Thread};
use std::time::{Duration, Instant};

use mpeg2ts::ts::{ReadTsPacket, TsPacketReader};
use prost::Message;

use grid::Grid as TransmitionMatrix;
use rayon::prelude::*;

const ROWS: usize = 6;
const COLS: usize = 6;
const ROW_FEC: usize = 3;
const COLS_FEC: usize = 3;

const SHARD_SIZE: usize = 256;
const DATA_SIZE: usize = 188;

const PACKET_PAYLOAD: usize = 5;
const MTU_SIZE: usize = 1500; // At the time of asserts with IrisPacket length is less than MTU, future will generate PACKET_PAYLOAD based on MTU

fn serialize_and_pad_data(data: &DataPacket) -> Vec<u8> {
    // Serialize Data
    let mut serialized_data = Vec::new();
    Message::encode(data, &mut serialized_data)
        .unwrap_or_else(|error| {
            eprintln!("Error: {}", error);
        });

    // Add padding SHARD_SIZE for FEC
    serialized_data.resize(SHARD_SIZE, 0);

    serialized_data
}


// This is running inside a separate task
pub fn start(file_path: &str, tx: Sender<Vec<u8>>) {
    // Open the file
    let file = match File::open(&file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening file: {}", e);
            return;
        }
    };
    let mut reader = BufReader::new(file);

    // Buffer to store the read bytes
    let mut buffer = [0; DATA_SIZE]; // Assuming you want to read exactly 188 bytes

    let mut global_matrix: Vec<TransmitionMatrix<DataPacket>> = Vec::new();
    let mut matrix_index = 1;

    let start_time = Instant::now();

    // give loop a name so the break exists multiple loops
    'main_loop: loop {
        // one iter is a matrix

        // init new matrix with size 0x0
        let mut matrix: TransmitionMatrix<DataPacket> = TransmitionMatrix::new(0, 0); 

        // Starting from rows because we are gonna transpose the matrix
        // 1 2 3    1 4 7
        // 4 5 6 => 2 5 8
        // 7 8 9    3 6 9
        for _ in 0..ROWS {
            let mut row = vec![];

            for _ in 0..COLS {

                // Fill buffer with contents of file 
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        println!("END OF FILE");

                        // End of file reached break the reading loop
                        break 'main_loop;
                    }
                    Ok(n) => {
                        // Process the read data here and create a DataPacket
                        let data = DataPacket {
                            matrix: matrix_index,
                            index: 0,
                            width: COLS as i32,
                            height: ROWS as i32,
                            fec: false,
                            data: buffer[..n].to_vec(),
                        };
                        // Add to row
                        row.push(data);
                    }
                    Err(e) => {
                        // Handle error
                        eprintln!("Error reading data from file: {}", e);
                        break;
                    }
                }

            }
            // Push row to matrix
            matrix.push_row(row);
        }

        // At this point Matrix if filled with DataPackets

        // Init buf containing all row fec
        let mut rows_fec = Vec::new();
        
        // Apply FEC to every row and save in a tmp var
        for row in matrix.iter_rows() {
            // Serialize the DataPacket Struct for FEC
            let serialized_row: Vec<Vec<u8>> = row.map(|row| {
                serialize_and_pad_data(row)
            }).collect();

            // Apply FEC
            let row_fec = reed_solomon_16::encode(
                serialized_row.len(), // total number of original shards
                ROW_FEC, // total number of recovery shards
                serialized_row, // all original shards
            ).unwrap();

            // Add to row_fec
            rows_fec.push(row_fec);
        }

        // Transpose the rows, appending columns to the matrix
        let transposed_fec: Vec<Vec<_>> = (0..rows_fec[0].len())
        .map(|row_index| {
            rows_fec.iter().map(|row| row[row_index].clone()).collect()
        })
        .collect();


        // Create the DataPackets with FEC Data
        let columns: Vec<Vec<DataPacket>> = transposed_fec.iter().map(|row: &Vec<Vec<u8>>| {
            // Iterate over every row 
            row.iter().map(|item: &Vec<u8>| {
                // Create a new DataPacket
                DataPacket {
                    matrix: matrix_index,
                    index: 0,
                    width: COLS as i32,
                    height: ROWS as i32,
                    fec: true,
                    data: item.clone(),
                }
            }).collect()
        }).collect();

        // Push collumns to matrix
        for coll in columns {
            matrix.push_col(coll);
        } 

        // Here the matrix is
        // 1 2 3 P P 
        // 4 5 6 P P
        // 7 8 9 P P

        // Init buf containing all column fec
        let mut cols_fec = Vec::new();

        // Apply FEC to every column and save in a tmp var
        for col in matrix.iter_cols() {
            // Serialize the DataPacket Struct for FEC
            let serialized_col: Vec<Vec<u8>> = col.map(|col| {
                serialize_and_pad_data(col)
            }).collect();

            // Apply FEC
            let col_fec = reed_solomon_16::encode(
                serialized_col.len(), // total number of original shards
                COLS_FEC, // total number of recovery shards for columns
                serialized_col, // all original shards
            ).unwrap();            

            // Add to cols_fec
            cols_fec.push(col_fec);
        }

        // Transpose the columns, appending rows to the matrix
        let transposed_cols_fec: Vec<Vec<_>> = (0..cols_fec[0].len())
            .map(|col_index| {
                cols_fec.iter().map(|col| col[col_index].clone()).collect()
            })
            .collect();

        // Create the DataPackets with FEC Data for columns
        let rows: Vec<Vec<DataPacket>> = transposed_cols_fec.iter().map(|col: &Vec<Vec<u8>>| {
            // Iterate over every column
            col.iter().map(|item: &Vec<u8>| {
                // Create a new DataPacket
                DataPacket {
                    matrix: matrix_index,
                    index: 0,
                    width: COLS as i32,
                    height: ROWS as i32,
                    fec: true,
                    data: item.clone(),
                }
            }).collect()
        }).collect();

        // Push rows to matrix
        for row in rows {
            matrix.push_row(row);
        }

        // Here the matrix is
        // 1 2 3 P P 
        // 4 5 6 P P
        // 7 8 9 P P
        // P P P P P
        // P P P P P       

        // Iterate for all matrix and send to multicast
        // THIS CAN BE IMPROVED (NEW THREAD TO PROCESS THIS, IT'S BLOCKING THE CREATION OF THE NEXT MATRIX)
        let mut packet_buffer = Vec::new();
        let mut index = 0;

        for packet in matrix.iter_mut() {

            // Update every DataPacket index position
            packet.index = index;
            index += 1; // Increment the index            

            // Add packet to the buffer
            packet_buffer.push(packet.clone());

            // Check if we have PACKET_PAYLOAD packets
            if packet_buffer.len() == PACKET_PAYLOAD {

                // Create IrisPacket with payload
                let iris_packet = IrisPacket {
                    payload: packet_buffer.clone()
                };
                
                // Serialize Data
                let mut serialized_data = Vec::new();
                Message::encode(&iris_packet, &mut serialized_data).unwrap_or_else(|error| {
                    eprintln!("Error: {}", error);
                });

                // Send the buffer
                tx.send(serialized_data).unwrap_or_else(|e| {
                    println!("error sending data {:?}", e.0);
                });

                // Reset buffer
                packet_buffer.clear();
            }

        }

        // Check if there are any remaining packets in the buffer after the loop
        if !packet_buffer.is_empty() {
            // Create IrisPacket with payload
            let iris_packet = IrisPacket {
                payload: packet_buffer
            };
            
            // Serialize Data
            let mut serialized_data = Vec::new();
            Message::encode(&iris_packet, &mut serialized_data).unwrap_or_else(|error| {
                eprintln!("Error: {}", error);
            });

            assert!(serialized_data.len() < MTU_SIZE);

            // Send the buffer
            tx.send(serialized_data).unwrap_or_else(|e| {
                println!("error sending data {:?}", e);
            });

        }


        global_matrix.push(matrix);
        matrix_index += 1;
    }
    let end_time = Instant::now();
    let duration = end_time.duration_since(start_time);

    println!("Time taken: {:?}", duration);
    println!("Last Matrix: {:?}", global_matrix.last().unwrap().get(0, 0).unwrap().matrix);

}