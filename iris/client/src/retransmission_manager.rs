use crossbeam_channel::Receiver;
use log::{error, info};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::proto::MeshPacket;

#[derive(Debug)]
pub struct RetransmissionManager {
    requests_channel: Receiver<MeshPacket>,
}
impl RetransmissionManager {
    pub fn new(requests_channel: Receiver<MeshPacket>) -> Self {
        Self { requests_channel }
    }

    /// Processes all active retransmission requests.
    pub fn process_requests(&self, shutdown_flag: Arc<AtomicBool>) {
        while !shutdown_flag.load(Ordering::Relaxed) {
            match self.requests_channel.recv() {
                Ok(packet) => {
                    // Process the packet, e.g., log it or handle retransmission logic
                    info!("{:?}", packet.message_id);
                }
                Err(_) => {
                    // Channel closed, exit the loop
                    info!("Retransmission requests channel closed.");
                    break;
                }
            }
        }
    }
}
