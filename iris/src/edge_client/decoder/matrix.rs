use std::collections::HashSet;
use std::error::Error;
use std::sync::Mutex;
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use log::error;
use prost::Message;

use crate::edge_client::core::proto::IrisPacket;

#[derive(Default)]
struct MatrixTracker {
    matrix: i32,
    received_indexes: HashSet<i32>,
    width: i32,
    height: i32,
}

impl MatrixTracker {
    fn new(matrix: i32, width: i32, height: i32) -> Self {
        Self {
            matrix,
            received_indexes: HashSet::new(),
            width,
            height,
        }
    }

    fn add_packet(&mut self, index: i32) {
        self.received_indexes.insert(index);
    }

    fn is_complete(&self) -> bool {
        self.received_indexes.len() as i32 == self.width * self.height
    }

    fn missing_indexes(&self) -> Vec<i32> {
        let mut missing = Vec::new();
        for i in 0..(self.width * self.height) {
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
        let matrix_cell = match receiver.recv() {
            Ok(matrix_cell) => matrix_cell,
            Err(err) => {
                error!("Receiver error: {}", err);
                return Err(Box::new(err));
            }
        };

        let deserialized_matrix_cell = match IrisPacket::decode(&*matrix_cell) {
            Ok(packet) => packet,
            Err(err) => {
                error!("Error decoding packet: {}", err);
                return Err(Box::new(err));
            }
        };

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
                    *tracker_guard = Some(MatrixTracker::new(packet.matrix, packet.width, packet.height));
                }
            } else {
                *tracker_guard = Some(MatrixTracker::new(packet.matrix, packet.width, packet.height));
            }
            
            if let Some(ref mut tracker) = *tracker_guard {
                tracker.add_packet(packet.index);
            }
            
            drop(tracker_guard);
            
            if !packet.fec {
                if let Err(err) = decode_data_sender.send(packet.data) {
                    error!("Error sending data: {}", err);
                    return Err(Box::new(err));
                }
            }
        }
    }
}
