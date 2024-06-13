use crossbeam_channel::Receiver;
use std::thread;

use super::wifi_multicast::WiFiMulticast;
use super::client_sessions::{self, ClientSessions};

// Struct representing the pipeline composed of the components
pub struct Pipeline {
    client_sessions: ClientSessions,
    wifi_multicast: WiFiMulticast,
    // wifi_unicast: Option<WiFiUnicast> // Optional
}

impl Pipeline {
    // Constructor to create a new instance of Pipeline
    pub fn new() -> Self {
        Pipeline {
            client_sessions: ClientSessions::new(),
            wifi_multicast: WiFiMulticast::new(10_000_000)
        }
    }

    /// Starts the pipeline with the provided source handler receiver.
    pub fn start(self, source_handler_receiver: Receiver<Vec<u8>>) {
        // Destructure self to take ownership of the components
        let Pipeline { mut client_sessions, mut wifi_multicast } = self;

        // Start client_sessions thread
        let client_sessions_handler = thread::spawn(move || {
            // Start client receiver reports server
            client_sessions.server();
        });

        // Start wifi_multicast thread
        let wifi_multicast_handler = thread::spawn(move || {
            // Start multicast sender with the received data
            wifi_multicast.server(source_handler_receiver);
        });

        // Wait for the threads to finish execution
        wifi_multicast_handler.join().expect("wifi_multicast_handler panicked");
        client_sessions_handler.join().expect("client_sessions_handler panicked");
    }
}
