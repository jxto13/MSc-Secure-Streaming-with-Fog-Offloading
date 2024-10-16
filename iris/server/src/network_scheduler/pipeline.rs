use crossbeam_channel::Receiver;

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::CONFIG;

use super::wifi_multicast::WiFiMulticast;
use crate::membership_manager::client_sessions::ClientSessions;
use super::wifi_unicast::WiFiUnicast;

// Struct representing the pipeline composed of the components
pub struct Pipeline {
    client_sessions: Arc<Mutex<ClientSessions>>,
    wifi_multicast: Arc<Mutex<WiFiMulticast>>,
    wifi_unicast: Arc<Mutex<WiFiUnicast>>
}

impl Pipeline {
    // Constructor to create a new instance of Pipeline
    pub fn new() -> Self {
        Pipeline {
            client_sessions: Arc::new(Mutex::new(ClientSessions::new())),
            wifi_multicast: Arc::new(Mutex::new(WiFiMulticast::new())),
            wifi_unicast: Arc::new(Mutex::new(WiFiUnicast::new()))
        }
    }

    /// Starts the pipeline with the provided source handler receiver.
    pub fn start(&self, source_handler_receiver: Receiver<Vec<u8>>) {
        let client_sessions = Arc::clone(&self.client_sessions);
        let wifi_multicast = Arc::clone(&self.wifi_multicast);
        let wifi_unicast = Arc::clone(&self.wifi_unicast);

        // Start client_sessions thread
        let client_sessions_handler = thread::spawn(move || {
            let timeout = Duration::from_secs(CONFIG.get().unwrap().settings.timeout_clients);
            // Check every second for timeout clients
            loop {
                {
                    // Lock only for the duration of removing timed-out clients
                    let mut sessions = client_sessions.lock().unwrap();
                    sessions.remove_timed_out_clients();
                }
                // Sleep outside the lock to avoid holding it during the sleep
                thread::sleep(timeout);
            }
        });

        // Start wifi_multicast thread
        let wifi_multicast_handler = thread::spawn(move || {
            // Start multicast sender with the received data
            wifi_multicast.lock().unwrap().server(source_handler_receiver).unwrap();
        });

        // Wait for the threads to finish execution
        wifi_multicast_handler.join().expect("wifi_multicast_handler panicked");
        client_sessions_handler.join().expect("client_sessions_handler panicked");
    }

    pub fn get_arc_client_sessions(&self) -> Arc<Mutex<ClientSessions>>{
        self.client_sessions.clone()
    }
}
