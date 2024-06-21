use std::collections::HashSet;
use std::error::Error;
use std::fs::OpenOptions;
use std::sync::{Arc, Mutex};
use std::io::Write;
use crossbeam_channel::{Receiver, Sender};
use log::{debug, error, info};
use mpeg2ts::pes::{PesPacketReader, ReadPesPacket};
use mpeg2ts::ts::{ReadTsPacket, TsPacketReader, TsPacketWriter, WriteTsPacket};
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

    fn update_indexes(&mut self, list_index: &[i32]) {
        for &index in list_index {
            // info!("recovered - {}", index);
            self.received_indexes.insert(index);
        }
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

    fn remove_index(&self, rows: &mut Vec<Vec<i32>>, cols: &mut Vec<Vec<i32>>, index: i32) {
        let row = index / self.cols;
        let col = index % self.cols;

        // Remove the index from the specified row in rows vector
        if let Some(pos) = rows[row as usize].iter().position(|&x| x == index) {
            rows[row as usize].remove(pos);
        }

        // Remove the index from the specified column in cols vector
        if let Some(pos) = cols[col as usize].iter().position(|&x| x == index) {
            cols[col as usize].remove(pos);
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

                            // Check for missing packets
                            if !tracker.is_complete()  {
                                // Record the start time
                                let start = std::time::Instant::now();
                                
                                for iteration in 0..2 {
                                    // info!("iteration number - {}", iteration);

                                    // this can be optimized 
                                    let missing_indexes = tracker.missing_indexes();
                                    // error!("Missing indexes from matrix {}: {:?}", tracker.matrix_index, missing_indexes);

                                    // this can be optimized 
                                    let (mut missing_rows, mut missing_cols) = tracker.group_missing_indexes();

                                    let mut recoverd_indexes = Vec::new();
                                    // Handle missing indexes in the same column
                                    for col in missing_cols.iter() {
                                        if col.len() >= 1 {
                                            // info!("Missing indexes in the same column: {:?}", col);

                                            let mut original_shards = Vec::new();
                                            let mut recovery_shards = Vec::new();

                                            for (col_position, item) in tracker.matrix.iter_col(col[0] as usize % tracker.cols as usize).enumerate() {
                                                // if index is not in missing array and its not an empty packet (len of payload is not 0)
                                                if !col.contains(&item.index) && item.payload.len() != 0 {

                                                    // if less the data row length is an original shard
                                                    if item.index / tracker.cols < (tracker.rows - tracker.cols_fec) {
                                                        original_shards.push((col_position, item.payload.clone()));
                                                    }

                                                    // if index is not in missing array and is larger then data row length is a recovery shard
                                                    if item.index / tracker.cols >= (tracker.rows - tracker.cols_fec) {
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
                                                    let col_index = col[0] as usize % tracker.cols as usize;

                                                    for (col_enumerator, item) in tracker.matrix.iter_col_mut(col_index).enumerate() {
                                                        if let Some(value) = recovered.get(&col_enumerator) {

                                                            let packet_index = col_index as i32 + tracker.cols * col_enumerator as i32;

                                                            // Update Matrix with the recovered data value
                                                            item.matrix = tracker.matrix_index;
                                                            item.index = packet_index;
                                                            item.rows = tracker.rows - tracker.cols_fec;
                                                            item.cols = tracker.cols - tracker.rows_fec;
                                                            item.rows_fec = tracker.rows_fec;
                                                            item.cols_fec = tracker.cols_fec;
                                                            item.fec = false;
                                                            item.payload = value.clone();

                                                            // Update tracker for missing indexes
                                                            recoverd_indexes.push(packet_index);

                                                            // info!("col - The value for {} is {:?}", col_index as i32 + tracker.cols * col_enumerator as i32, value);
                                                        } 
                                                    }
                                                    tracker.update_indexes(&recoverd_indexes);

                                                   

                                                },
                                                Err(err) => {
                                                    // error!("{}", err);
                                                },
                                            };
                                        }
                                    }

                                    for index in recoverd_indexes  {
                                        tracker.remove_index(&mut missing_rows, &mut missing_cols, index);
                                    }

                                    // Handle missing indexes in the same row
                                    for row in missing_rows.iter() {
                                        if row.len() >= 1 {
                                            // info!("Missing indexes in the same row: {:?}", row);

                                            let mut original_shards =  Vec::new();
                                            let mut recovery_shards = Vec::new();

                                            for (row_position, item) in tracker.matrix.iter_row(row[0] as usize / tracker.cols as usize).enumerate() {

                                                // if index is not in missing array and its not an empty packet (len of payload is not 0)
                                                if !row.contains(&item.index) && item.payload.len() != 0 {

                                                    // if less the data row length is an original shard
                                                    if item.index % tracker.cols < (tracker.cols - tracker.rows_fec) {
                                                        original_shards.push((row_position, item.payload.clone()));
                                                    }

                                                    // if index is not in missing array and is larger then data row length is an recovery shard
                                                    if item.index % tracker.cols >= (tracker.cols - tracker.rows_fec) {    
                                                        // The recovery shard is fully inside payload of Packet, this way it recovers the original packet
                                                        recovery_shards.push((row_position % (tracker.cols - tracker.rows_fec) as usize, item.payload.clone()));
                                                    }
                                                }
                                                
                                            }
                                            
                                            match reed_solomon_16::decode(
                                                tracker.cols as usize - tracker.rows_fec as usize, // total number of original shards
                                                tracker.rows_fec as usize, // total number of recovery shards
                                                original_shards,
                                                recovery_shards,
                                            ) {
                                                Ok(recovered) => {
                                                    let row_index = row[0] as usize / tracker.cols as usize;
                                                    let mut recoverd_indexes = Vec::new();

                                                    for (row_enumerator, item) in tracker.matrix.iter_row_mut(row_index).enumerate() {
                                                        if let Some(value) = recovered.get(&row_enumerator) {

                                                            let packet_index = row_index as i32 * tracker.cols + row_enumerator as i32;
                                                            // Update Matrix with the recovered data value
                                                            item.matrix = tracker.matrix_index;
                                                            item.index = packet_index;
                                                            item.rows = tracker.rows - tracker.cols_fec;
                                                            item.cols = tracker.cols - tracker.rows_fec;
                                                            item.rows_fec = tracker.rows_fec;
                                                            item.cols_fec = tracker.cols_fec;
                                                            item.fec = false;
                                                            item.payload = value.clone();

                                                            // Update tracker for missing indexes
                                                            recoverd_indexes.push(packet_index);

                                                            // info!("row - The value for {} is {:?}",row_index as i32 * tracker.cols + row_enumerator as i32, value);
                                                        } 
                                                    }

                                                    tracker.update_indexes(&recoverd_indexes);

                                                },
                                                Err(err) => {
                                                    // error!("{}",err);
                                                },
                                            };
                                        }
                                    }
                                }

                                // Print the elapsed time
                                info!("Time elapsed for reconstructing matrix {:?}: {:?}", tracker.matrix_index, start.elapsed());
                            }

                            for _ in 0..tracker.rows_fec {
                                tracker.matrix.pop_col();
                            }

                            for _ in 0..tracker.cols_fec {
                                tracker.matrix.pop_row();
                            }

                            for item in tracker.matrix.iter(){
                                decode_data_sender.send(item.payload.clone()).unwrap();
                            }

                            // Update tracker for new matrix
                            *tracker_guard = Some(MatrixTracker::new(&packet));
                        } 
                    } else {
                        *tracker_guard = Some(MatrixTracker::new(&packet));
                    }

                    if let Some(ref mut tracker) = *tracker_guard {
                        tracker.add_packet(packet.clone());
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
