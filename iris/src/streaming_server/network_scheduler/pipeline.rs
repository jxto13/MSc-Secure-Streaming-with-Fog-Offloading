use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread;

use super::wifi_multicast::WiFiMulticast;

// Struct representing the pipeline composed of the components
pub struct Pipeline {
    // Client sessions
    wifi_multicast: Arc<Mutex<WiFiMulticast>>,
    // wifi_unicast: Option<WiFiUnicast> // Optional
}

impl Pipeline {
    // Constructor to create a new instance of Pipeline
    pub fn new() -> Self {
        Pipeline { 
            wifi_multicast: Arc::new(Mutex::new(WiFiMulticast::new()))
        }
    }

    /// Starts the pipeline with the provided source handler receiver.
    pub fn start(&self, source_handler_receiver: Receiver<Vec<u8>>) {

        // Clone the Arc<Mutex<WiFiMulticast>> for the thread
        let wifi_multicast_clone = self.wifi_multicast.clone();

        // Start wifi_multicast thread
        let wifi_multicast_handler = thread::spawn(move || {
            // Lock the mutex to access WiFiMulticast
            let wifi_multicast = wifi_multicast_clone.lock().unwrap();
            // Start multicast sender with the received data
            wifi_multicast.start_multicast_sender(source_handler_receiver);
        });
        
        // Wait for the thread to finish execution
        wifi_multicast_handler.join().expect("wifi_multicast_handler panicked");
    }
}
