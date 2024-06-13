use std::collections::HashSet;
use std::error::Error;
use std::sync::Mutex;
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use log::error;
use log::info;
use prost::Message;

use crate::edge_client::core::proto::{TransmitionPacket, IrisPacket, ItemType};
use reed_solomon_16::{ReedSolomonDecoder};
use grid::Grid as Matrix;

#[derive(Default)]
struct MatrixTracker {
    matrix: i32,
    received_indexes: HashSet<i32>,
    cols: i32,
    rows: i32,
    packets: Vec<Option<TransmitionPacket>>,
}

impl MatrixTracker {
    fn new(matrix: i32, cols: i32, rows: i32) -> Self {
        Self {
            matrix,
            received_indexes: HashSet::new(),
            cols,
            rows,
            packets: vec![None; (cols * rows) as usize],
        }
    }

    fn add_packet(&mut self, packet: TransmitionPacket) {
        let index = packet.index as usize;
        self.received_indexes.insert(packet.index);
        self.packets[index] = Some(packet);
    }

    fn is_complete(&self) -> bool {
        self.received_indexes.len() as i32 == self.cols * self.rows
    }

    fn missing_indexes(&self) -> Vec<i32> {
        let mut missing = Vec::new();
        for i in 0..(self.cols * self.rows) {
            if !self.received_indexes.contains(&i) {
                missing.push(i);
            }
        }
        missing
    }
}

pub fn start(receiver: Receiver<Vec<u8>>, decode_data_sender: Sender<Vec<u8>>) -> Result<(), Box<dyn Error>> {
    let current_tracker: Arc<Mutex<Option<MatrixTracker>>> = Arc::new(Mutex::new(None));

    loop {
        // Can add multiple receivers here, will add unicast receiver because this is non-block, only will try to receive 
        // Multicast Receiver 
        match receiver.try_recv() {
            Ok(matrix_cell) => {
                let deserialized_matrix_cell = match IrisPacket::decode(&*matrix_cell) {
                    Ok(packet) => packet,
                    Err(err) => {
                        error!("Error decoding packet: {}", err);
                        continue;
                    }
                };

                // This is where i will verify signature, if not correct then drop cell 

                for packet in deserialized_matrix_cell.payload {
                    let mut tracker_guard = current_tracker.lock().unwrap();

                    if let Some(ref mut tracker) = *tracker_guard {
                        if tracker.matrix != packet.matrix {
                            // Check for missing packets
                            if !tracker.is_complete() {
                                let missing_indexes = tracker.missing_indexes();
                                error!("Missing indexes from matrix {}: {:?}", tracker.matrix, missing_indexes);
                            }
                            // Update tracker for new matrix
                            *tracker_guard = Some(MatrixTracker::new(packet.matrix, packet.cols, packet.rows));
                        }
                    } else {
                        *tracker_guard = Some(MatrixTracker::new(packet.matrix, packet.cols, packet.rows));
                    }

                    if let Some(ref mut tracker) = *tracker_guard {
                        tracker.add_packet(packet);
                    }

                    drop(tracker_guard);
                }
            },
            Err(err) => {
                error!("Receiver error: {}", err);
                continue;
            }
        }
    }
}
