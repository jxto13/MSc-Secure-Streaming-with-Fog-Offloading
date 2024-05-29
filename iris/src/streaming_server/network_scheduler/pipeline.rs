
use crossbeam_channel::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;

use super::wifi_multicast::WiFiMulticast;
use super::client_sessions::{self, ClientSessions};

// Struct representing the pipeline composed of the components
pub struct Pipeline {
    client_sessions: Arc<Mutex<ClientSessions>>,
    pub wifi_multicast: Arc<Mutex<WiFiMulticast>>,
    // wifi_unicast: Option<WiFiUnicast> // Optional
}

impl Pipeline {
    // Constructor to create a new instance of Pipeline
    pub fn new() -> Self {
        Pipeline {
            client_sessions: Arc::new(Mutex::new(ClientSessions::new())),
            wifi_multicast: Arc::new(Mutex::new(WiFiMulticast::new(100_000_000)))
        }
    }

    /// Starts the pipeline with the provided source handler receiver.
    pub fn start(&self, source_handler_receiver: Receiver<Vec<u8>>) {

        // Clone the Arc<Mutex<ClientSessions>> for the thread
        let client_sessions_clone = self.client_sessions.clone();

        // Start client_sessions thread
        let client_sessions_handler = thread::spawn(move ||{
            // Lock the mutex to access client_sessions
            let mut client_sessions = client_sessions_clone.lock().unwrap();
            // Start client receiver reports server
            client_sessions.start_receiver_server();
        });


        // Clone the Arc<Mutex<WiFiMulticast>> for the thread
        let wifi_multicast_clone = self.wifi_multicast.clone();

        // Start wifi_multicast thread
        let wifi_multicast_handler = thread::spawn(move || {
            // Lock the mutex to access WiFiMulticast
            let mut wifi_multicast = wifi_multicast_clone.lock().unwrap();
            // Start multicast sender with the received data
            wifi_multicast.start_multicast_sender(source_handler_receiver);
        });
        
        // Wait for the thread to finish execution
        wifi_multicast_handler.join().expect("wifi_multicast_handler panicked");
        client_sessions_handler.join().expect("client_sessions_handler panicked")
    }
}
