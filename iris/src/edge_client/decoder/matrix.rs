
use std::error::Error;

use crossbeam_channel::{Receiver, Sender};

use log::error;
use prost::Message;

use crate::edge_client::core::proto::IrisPacket;

use crossbeam_queue::SegQueue;

pub fn start(receiver: Receiver<Vec<u8>>, decode_data_sender: Sender<Vec<u8>>) -> Result<(), Box<dyn Error>> {
    
    // let q = SegQueue::new();

    loop {

        // Blocking until cells in the channel
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
            if !packet.fec {
                // q.push(packet.data);
                // if let Err(err) = decode_data_sender.send(packet.data) {
                //     error!("Error sending data: {}", err);
                //     // Optionally handle the error, such as logging it or returning it
                //     return Err(Box::new(err));
                // }
            }
        }
    }
}
