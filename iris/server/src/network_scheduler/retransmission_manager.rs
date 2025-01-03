use std::{net::SocketAddr, sync::{Arc, Mutex, RwLock}, time::Duration};

use chrono::Utc;
use circular_buffer::CircularBuffer;
use crossbeam_channel::Sender;
use grid::Grid;
use log::info;
use prost::Message;
use crate::{proto::RetransmissionPacket, TransmissionPacket};
use crate::{membership_manager::client_sessions::{self, ClientSessions}, MATRIX_BUFFER};




// Retransmission manager struct
pub struct RetransmissionManager {
    
}

impl RetransmissionManager {
    // Initialize the manager with receivers for multicast and unicast retransmissions
    pub fn new() -> Self {
        Self {
        }
    }

    // Run the retransmission handler that listens to both channels
    pub fn handle_retransmissions(client_sessions: Arc<RwLock<ClientSessions>>, multicast_tx: Sender<Vec<u8>>, unicast_tx: Sender<Vec<u8>>) {


        let retransmission_interval = Duration::from_millis(100); // Define the interval for retransmission rounds

        loop {
            std::thread::sleep(retransmission_interval);

        
        }

    }

    fn encoder() {

    }

}