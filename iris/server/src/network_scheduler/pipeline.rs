use crossbeam_channel::{unbounded, Receiver};

use std::sync::{Arc, RwLock};
use std::thread;


use crate::membership_manager::client_sessions::ClientSessions;
use super::wifi_multicast::WiFiMulticast;
use super::wifi_unicast::WiFiUnicast;
use super::retransmission_manager::RetransmissionManager;

// Struct representing the pipeline composed of the components
pub struct Pipeline {
    client_sessions: Arc<RwLock<ClientSessions>>,
    wifi_multicast_retransmission: bool,
    wifi_unicast_retransmission: bool,
    retransmission_manager: Option<RetransmissionManager>
}

impl Pipeline {
    // Constructor to create a new instance of Pipeline
    pub fn new(client_sessions: Arc<RwLock<ClientSessions>>, multicast_enabled: bool, unicast_enabled: bool) -> Self {
        Pipeline {
            client_sessions,
            wifi_multicast_retransmission: multicast_enabled,
            wifi_unicast_retransmission: unicast_enabled,
            retransmission_manager: None
        }
    }

    /// Starts the pipeline with the provided source handler receiver.
    pub fn start(&self, source_handler_receiver: Receiver<Vec<u8>>) {
        
        // Create channels for retransmission
        let (retransmission_multicast_tx, retransmission_multicast_rx) = unbounded();
        let (retransmission_unicast_tx, retransmission_unicast_rx) = unbounded();
        
        

        // Start wifi_multicast thread
        let wifi_multicast_handler = thread::spawn(move || {
            // Create a new instance of WiFiMulticast
            let mut multicast = WiFiMulticast::new();
            // Start multicast sender with the received data
            multicast.server(source_handler_receiver, retransmission_multicast_rx).unwrap();
        });

        
        // Start wifi_unicast thread
        let wifi_unicast_handler = thread::spawn(move || {
            // Start unicast sender
            let mut unicast = WiFiUnicast::new();
            unicast.server(retransmission_unicast_rx).unwrap();
        });

        // Start Retransmition Handler thread
        let retransmitions_handler = thread::spawn({
            let client_sessions = Arc::clone(&self.client_sessions);
            move || {
                // Start unicast sender
                RetransmissionManager::handle_retransmissions(client_sessions, retransmission_multicast_tx.clone(), retransmission_unicast_tx.clone());
            }
        });

        // Wait for the threads to finish execution
        {
            wifi_multicast_handler.join().expect("wifi_multicast_handler panicked");
            wifi_unicast_handler.join().expect("wifi_unicast_handler panicked");
            retransmitions_handler.join().expect("retransmitions_handler panicked");
        }
    }

    pub fn get_client_sessions(&self) -> Arc<RwLock<ClientSessions>>{
        Arc::clone(&self.client_sessions)
    }
}
