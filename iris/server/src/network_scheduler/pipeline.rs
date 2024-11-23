use crossbeam_channel::{unbounded, Receiver};

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::CONFIG;

use crate::membership_manager::client_sessions::ClientSessions;
use super::wifi_multicast::WiFiMulticast;
use super::wifi_unicast::WiFiUnicast;
use super::retransmission_manager::RetransmissionManager;

// Struct representing the pipeline composed of the components
pub struct Pipeline {
    client_sessions: Arc<Mutex<ClientSessions>>,
    wifi_multicast: Arc<Mutex<WiFiMulticast>>,
    wifi_unicast: Arc<Mutex<WiFiUnicast>>,
    retransmission_manager: Arc<Mutex<RetransmissionManager>>
}

impl Pipeline {
    // Constructor to create a new instance of Pipeline
    pub fn new() -> Self {
        Pipeline {
            client_sessions: Arc::new(Mutex::new(ClientSessions::new())),
            wifi_multicast: Arc::new(Mutex::new(WiFiMulticast::new())),
            wifi_unicast: Arc::new(Mutex::new(WiFiUnicast::new())),
            retransmission_manager: Arc::new(Mutex::new(RetransmissionManager::new()))
        }
    }

    /// Starts the pipeline with the provided source handler receiver.
    pub fn start(&self, source_handler_receiver: Receiver<Vec<u8>>) {
        let client_sessions = Arc::clone(&self.client_sessions);

        let (retransmission_multicast_tx, retransmission_multicast_rx) = unbounded();
        let (retransmission_unicast_tx, retransmission_unicast_rx) = unbounded();

        let clone_client_sessions = self.client_sessions.clone();
        // Start client_sessions thread
        let client_sessions_handler = thread::spawn(move || {
            let timeout = Duration::from_secs(CONFIG.get().unwrap().settings.timeout_clients);
            // Check every N second for timeout clients
            loop {
                {
                    // Lock only for the duration of removing timed-out clients
                    let mut sessions = clone_client_sessions.lock().unwrap();
                    sessions.remove_timed_out_clients();
                }
                // Sleep outside the lock to avoid holding it during the sleep
                thread::sleep(timeout);
            }
        });

        // TODO: Refactor this code to use Options in pipeline and not lock the mutex permant in the thread

        // This clone is not relevant and be used the self and be consumed because this makes a permanent lock on mutex
        let clone_wifi_multicast = self.wifi_multicast.clone();
        // Start wifi_multicast thread
        let wifi_multicast_handler = thread::spawn(move || {
            // Start multicast sender with the received data
            clone_wifi_multicast.lock().unwrap().server(source_handler_receiver, retransmission_multicast_rx).unwrap();
        });

        // This clone is not relevant and be used the self and be consumed because this makes a permanent lock on mutex
        let clone_wifi_unicast = self.wifi_unicast.clone();
        // Start wifi_unicast thread
        let wifi_unicast_handler = thread::spawn(move || {
            // Start unicast sender
            clone_wifi_unicast;
        });

        // This clone is not relevant and be used the self and be consumed because this makes a permanent lock on mutex
        let clone_retransmission_manager = self.retransmission_manager.clone();
        let clone_client_sessions = self.client_sessions.clone();
        // Start Retransmition Handler thread
        let retransmitions_handler = thread::spawn(move || {
            // Start unicast sender
            clone_retransmission_manager.lock().unwrap().handle_retransmissions(clone_client_sessions, retransmission_multicast_tx.clone(), retransmission_unicast_tx.clone());
        });

        // Wait for the threads to finish execution
        wifi_multicast_handler.join().expect("wifi_multicast_handler panicked");
        wifi_unicast_handler.join().expect("wifi_unicast_handler panicked");
        client_sessions_handler.join().expect("client_sessions_handler panicked");
        retransmitions_handler.join().expect("retransmitions_handler panicked");
    }

    pub fn get_arc_client_sessions(&self) -> Arc<Mutex<ClientSessions>>{
        self.client_sessions.clone()
    }
}
