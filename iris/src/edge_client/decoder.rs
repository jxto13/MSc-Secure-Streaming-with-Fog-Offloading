use std::collections::HashSet;
use std::error::Error;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};
use log::{debug, error, info};
use prost::Message;

use crate::edge_client::proto::{IrisPacket, TransmitionPacket};
use reed_solomon_16::ReedSolomonDecoder;
use grid::Grid as Matrix;

const SHARD_SIZE: usize = 256;

#[derive(Default)]
struct MatrixTracker {
    matrix_index: i32,
    received_indexes: HashSet<i32>,
    cols: i32,
    rows: i32,
    cols_fec: i32,
    rows_fec: i32,
    matrix: Matrix<TransmitionPacket>,
}

impl MatrixTracker {
    fn new(packet: &TransmitionPacket) -> Self {
        let total_cols = packet.cols + packet.rows_fec;
        let total_rows = packet.rows + packet.cols_fec;
        Self {
            matrix_index: packet.matrix,
            received_indexes: HashSet::new(),
            cols: total_cols,
            rows: total_rows,
            cols_fec: packet.cols_fec,
            rows_fec: packet.rows_fec,
            matrix: Matrix::new(total_rows as usize, total_cols as usize),
        }
    }

    // Add packet to Tracker
    fn add_packet(&mut self, packet: TransmitionPacket) {
        let index = packet.index as usize;
        self.received_indexes.insert(packet.index);
        let row = index / self.cols as usize;
        let col = index % self.cols as usize;
        self.matrix[(row, col)] = packet;
    }

    // Checks if matrix is complete or not
    fn is_complete(&self) -> bool {
        self.received_indexes.len() as i32 == self.cols * self.rows
    }

    // Returns the missing index (the index are in row major) as vector
    fn missing_indexes(&self) -> Vec<i32> {
        (0..(self.cols * self.rows))
            .filter(|i| !self.received_indexes.contains(i))
            .collect()
    }

    // Groups the indexes by row and collumn to apply FEC decoder
    fn group_missing_indexes(&self) -> (Vec<Vec<i32>>, Vec<Vec<i32>>) {
        let mut rows: Vec<Vec<i32>> = vec![Vec::new(); self.rows as usize];
        let mut cols: Vec<Vec<i32>> = vec![Vec::new(); self.cols as usize];

        for &index in self.missing_indexes().iter() {
            let row = index / self.cols;
            let col = index % self.cols;
            rows[row as usize].push(index);
            cols[col as usize].push(index);
        }

        (rows, cols)
    }

    fn remove_restored_indexes(&self, rows: &mut Vec<Vec<i32>>, cols: &mut Vec<Vec<i32>>, indexes: Vec<i32>) {
        
        for &index in indexes.iter() {
            let row = index / self.cols;
            let col = index % self.cols;
    
            if let Some(pos) = rows[row as usize].iter().position(|&x| x == index) {
                rows[row as usize].remove(pos);
            }
    
            if let Some(pos) = cols[col as usize].iter().position(|&x| x == index) {
                cols[col as usize].remove(pos);
            }
        }
    }

    fn update_with_restored_packets(&mut self, restored_packets: Vec<TransmitionPacket>) {
        let mut indexes_restored = Vec::new();

        for packet in restored_packets {
            indexes_restored.push(packet.index);
            self.add_packet(packet);
        }
    }

}

pub fn start(receiver: Receiver<Vec<u8>>, decode_data_sender: Sender<Vec<u8>>) -> Result<(), Box<dyn Error>> {
    let current_tracker: Arc<Mutex<Option<MatrixTracker>>> = Arc::new(Mutex::new(None));

    loop {
        match receiver.try_recv() {
            Ok(matrix_cell) => {
                let deserialized_matrix_cell = match IrisPacket::decode(&*matrix_cell) {
                    Ok(packet) => packet,
                    Err(err) => {
                        error!("Error decoding packet: {}", err);
                        continue;
                    }
                };

                for packet in deserialized_matrix_cell.payload {
                    let mut tracker_guard = current_tracker.lock().unwrap();

                    if let Some(ref mut tracker) = *tracker_guard {
                        if tracker.matrix_index != packet.matrix {

                            // if tracker.matrix_index == 0 {
                            //     info!("{:?}", tracker.matrix);
                            //     let mut original_shards = Vec::new();
                            //     let mut recovery_shards = Vec::new();

                            //     let original_index = [0,1,3,4];
                            //     let recovery_index = [6,8];

                            //     // get first row and simulate packet lost
                            //     for (index, item) in tracker.matrix.iter_row(0).enumerate() {

                            //         if original_index.contains(&index){
                            //             // Serialize Data
                            //             // let mut serialized_data = Vec::new();
                            //             let mut serialized_data = item.encode_length_delimited_to_vec();
                                        
                            //             // let mut serialized_data = item.encode_length_delimited_to_vec();
                            //             // let size_option = serialized_data.get(0..2);
                            //             // let size = match size_option {
                            //             //     Some(size_in_bytes) => {
                                                
                            //             //     },
                            //             //     None => error!("array dosnt have 2 elements"),
                            //             // };
                            //             info!("serialized_data - {}",   serialized_data.len());
                            //             serialized_data.resize(SHARD_SIZE, 0);
                                        
                            //             original_shards.push((index, serialized_data))
                            //         }

                            //         if recovery_index.contains(&index){
                            //             recovery_shards.push((index - 6, item.payload.clone()));
                            //         }
                            //     }

                            //     match reed_solomon_16::decode(
                            //         tracker.cols as usize - tracker.rows_fec as usize, // total number of original shards
                            //         tracker.rows_fec as usize, // total number of recovery shards
                            //         original_shards,
                            //         recovery_shards,
                            //     ) {
                            //         Ok(recovered) => {
                            //             for item in recovered {

                            //                 info!("recovered: {} - {:?}",item.0, item.1);

                            //                 match TransmitionPacket::decode_length_delimited(&*item.1) {
                            //                     Ok(packet) => {
                            //                         info!("recovered: {:?}",packet);
                                                    
                            //                     },
                            //                     Err(err) => {
                            //                         error!("Error decoding packet: {}", err);
                            //                     }
                            //                 };
                            //             }
                            //         },
                            //         Err(err) => error!("{}",err),
                            //     };
                            // }

                            // Check for missing packets
                            if !tracker.is_complete()  {
                                // Record the start time
                                let start = std::time::Instant::now();

                                
                                // this can be optimized 
                                let missing_indexes = tracker.missing_indexes();
                                error!("Missing indexes from matrix {}: {:?}", tracker.matrix_index, missing_indexes);

                                // this can be optimized 
                                let (missing_rows, missing_cols) = tracker.group_missing_indexes();

                                // Handle missing indexes in the same column
                                for col in missing_cols {
                                    if col.len() >= 1 {
                                        info!("Missing indexes in the same column: {:?}", col);

                                        let mut original_shards = Vec::new();
                                        let mut recovery_shards = Vec::new();

                                        for (col_position, item) in tracker.matrix.iter_col(col[0] as usize % tracker.cols as usize).enumerate() {
                                            // if index is not in missing array and its not an empty packet (len of payload is not 0)
                                            if !col.contains(&item.index) && item.payload.len() != 0 {

                                                // if less the data row length is an original shard
                                                if item.index / tracker.cols < (tracker.rows - tracker.cols_fec) {
                                                    // info!("col_position = {}, original_shard = {:?}", col_position, item);

                                                    let mut serialized_data = item.encode_length_delimited_to_vec();

                                                    // Pad the serialized data to SHARD_SIZE len
                                                    serialized_data.resize(SHARD_SIZE, 0);

                                                    original_shards.push((col_position, serialized_data));
                                                }

                                                // if index is not in missing array and is larger then data row length is a recovery shard
                                                if item.index / tracker.cols >= (tracker.rows - tracker.cols_fec) {
                                                    // info!("position = {}, recovery_shard = {:?}", col_position % (tracker.rows - tracker.cols_fec) as usize, item.payload);

                                                    recovery_shards.push((col_position % (tracker.rows - tracker.cols_fec) as usize, item.payload.clone()));
                                                }
                                            }
                                        }

                                        match reed_solomon_16::decode(
                                            tracker.rows as usize - tracker.cols_fec as usize, // total number of original shards
                                            tracker.cols_fec as usize, // total number of recovery shards
                                            original_shards,
                                            recovery_shards,
                                        ) {
                                            Ok(recovered) => {
                                                for (index, item) in recovered {
                                                    
                                                    match TransmitionPacket::decode_length_delimited(&*item) {
                                                        Ok(packet) => {
                                                            info!("recovered: {:?}", packet);
                                                        },
                                                        Err(err) => {
                                                            error!("Error decoding packet index {}: {}", index, err);
                                                        }
                                                    };
                                                }
                                            },
                                            Err(err) => error!("{}", err),
                                        };
                                    }
                                }

                                // Handle missing indexes in the same row
                                for row in missing_rows {
                                    if row.len() >= 1 {
                                        info!("Missing indexes in the same row: {:?}", row);

                                        let mut original_shards =  Vec::new();
                                        let mut recovery_shards = Vec::new();

                                        for (row_position, item) in tracker.matrix.iter_row(row[0] as usize / tracker.cols as usize).enumerate() {

                                            // if index is not in missing array and its not an empty packet (len of payload is not 0)
                                            if !row.contains(&item.index) && item.payload.len() != 0 {

                                                // info!("row = {:?}, &item.index = {}, row.contains(&item.index) = {}", row, &item.index, row.contains(&item.index));
                                                // if less the data row length is an original shard
                                                if item.index % tracker.cols < (tracker.cols - tracker.rows_fec) {
                                                    // info!("row_position = {}, original_shard = {:?}", row_position, item);
                                                    
                                                    // Serialize Data
                                                    // let mut serialized_data = Vec::new();
                                                    // Message::encode(item, &mut serialized_data).unwrap_or_else(|error| {
                                                    //     eprintln!("Error: {}", error);
                                                    // });

                                                    let mut serialized_data = item.encode_length_delimited_to_vec();
                                                    

                                                    // Pad the serialized data to SHARD_SIZE len
                                                    serialized_data.resize(SHARD_SIZE, 0);

                                                    original_shards.push((row_position, serialized_data));
                                                }

                                                // if index is not in missing array and is larger then data row length is an recovery shard
                                                if item.index % tracker.cols >= (tracker.cols - tracker.rows_fec) {
                                                    // info!("position = {}, recovery_shard = {:?}", row_position % (tracker.cols - tracker.rows_fec) as usize, item.payload);
    
                                                    // The recovery shard is fully inside payload of Packet, this way it recovers the original packet
                                                    recovery_shards.push((row_position % (tracker.cols - tracker.rows_fec) as usize, item.payload.clone()));
                                                }
                                            }
                                            
                                            // debug!("original = {:?}, recovery = {:?}", original_shards, recovery_shards);
                                            // info!("row = {}, item = {:?}", row_position, item);
                                        }
                                        // info!("original = {:?}", original_shards);

                                        
                                        match reed_solomon_16::decode(
                                            tracker.cols as usize - tracker.rows_fec as usize, // total number of original shards
                                            tracker.rows_fec as usize, // total number of recovery shards
                                            original_shards,
                                            recovery_shards,
                                        ) {
                                            Ok(recovered) => {
                                                for item in recovered {

                                                    match TransmitionPacket::decode_length_delimited(&*item.1) {
                                                        Ok(packet) => {
                                                            info!("recovered: {:?}",packet);
                                                            
                                                        },
                                                        Err(err) => {
                                                            // error!("Error decoding packet: {}", err);
                                                        }
                                                    };
                                                }
                                            },
                                            Err(err) => error!("{}",err),
                                        };
                                    }
                                }

                                

                                
                                // Print the elapsed time
                                info!("Time elapsed for reconstructing matrix {:?}: {:?}", tracker.matrix_index, start.elapsed());
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
            },
            Err(err) => {
                if !err.is_empty() {
                    error!("Receiver error: {}", err);
                }
            }
        }
    }
}
