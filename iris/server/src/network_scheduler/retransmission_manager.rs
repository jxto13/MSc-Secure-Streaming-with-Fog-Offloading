use std::sync::{Arc, Mutex};

use chrono::Utc;
use crossbeam_channel::Sender;
use log::info;

use crate::membership_manager::client_sessions::{self, ClientSessions};




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
    pub fn handle_retransmissions(&self, client_sessions: Arc<Mutex<ClientSessions>>, multicast_tx: Sender<Vec<u8>>, unicast_tx: Sender<Vec<u8>>) {

        let mut start = Utc::now();
        let timout = chrono::Duration::seconds(1);

        loop {
            let now = Utc::now();

            if now - start >= timout {
                // for client in &client_sessions.lock().unwrap().clients {
                //     info!("Client {:?}: {:?}", client.0, client.1.reports.len());
                // }
                info!("Clients: {:?}", client_sessions.lock().unwrap().clients.len());

                start = now;
            }


        }
    }

    fn encoder() {

    }

}