use crossbeam_channel::{unbounded, Receiver};

use std::sync::Arc;
use std::thread;


use crate::membership_manager::client_sessions::ClientSessions;
use crate::network_scheduler;
use super::wifi_multicast::WiFiMulticast;
use super::wifi_unicast::WiFiUnicast;
use super::retransmission_manager::RetransmissionManager;
use network_scheduler::retransmission_manager::Phase;


// Struct representing the pipeline composed of the components
pub struct Pipeline {
}

impl Pipeline {

    /// Starts the pipeline with the provided source handler receiver.
    pub fn start(client_sessions: Arc<ClientSessions>, source_handler_receiver: Receiver<Vec<u8>>) {
        
        // Create channels for retransmission
        let (retransmission_multicast_tx, retransmission_multicast_rx) = unbounded();
        let (retransmission_unicast_tx, retransmission_unicast_rx) = unbounded();
        
        // Create a vector to hold the thread handlers
        let mut handlers = Vec::new();

        // Start multicast sender thread
        handlers.push(("wifimulticast_handler", thread::spawn(move || {
            // Create a new instance of WiFiMulticast
            let mut multicast = WiFiMulticast::new();
            // Start multicast sender with the received data
            multicast.server(source_handler_receiver, retransmission_multicast_rx).unwrap();
        })));
    

        // Start unicast sender thread
        handlers.push(("unicast_handler", thread::spawn(move || {
            // Create a new instance of WiFiUnicast
            let mut unicast = WiFiUnicast::new();
            // Start unicast sender with the received data
            unicast.server(retransmission_unicast_rx).unwrap();
        })));
        
        // Start unicast sender
        let mut manager = RetransmissionManager::new(vec![Phase::Multicast, Phase::Unicast]);
        manager.handle_retransmissions(client_sessions, retransmission_multicast_tx.clone(), retransmission_unicast_tx.clone());
    }

}
