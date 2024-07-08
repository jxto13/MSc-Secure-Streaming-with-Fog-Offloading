use crossbeam_channel::{unbounded, Receiver};
use iris::streaming_server::proto::report_service_server::ReportServiceServer;
use log::info;
use std::sync::{Arc, Mutex};
use std::{io, thread};
use std::time::Duration;

use super::wifi_multicast::WiFiMulticast;
use super::client_sessions::{self, ClientSessions, MyReportService};

// Struct representing the pipeline composed of the components
pub struct Pipeline {
    client_sessions: Arc<Mutex<ClientSessions>>,
    wifi_multicast: Arc<Mutex<WiFiMulticast>>,
    // wifi_unicast: Option<WiFiUnicast> // Optional
}

impl Pipeline {
    // Constructor to create a new instance of Pipeline
    pub fn new() -> Self {
        Pipeline {
            client_sessions: Arc::new(Mutex::new(ClientSessions::new())),
            wifi_multicast: Arc::new(Mutex::new(WiFiMulticast::new(10_000_000))),
        }
    }

    pub fn get_client_sessions_service(&self) -> ReportServiceServer<MyReportService>{
        ReportServiceServer::new(MyReportService::new(Arc::clone(&self.client_sessions)))
    }

    /// Starts the pipeline with the provided source handler receiver.
    pub fn start(&self, source_handler_receiver: Receiver<Vec<u8>>) {
        let client_sessions = Arc::clone(&self.client_sessions);
        let wifi_multicast = Arc::clone(&self.wifi_multicast);

        info!("Press Enter to continue...");
        
        // Create a buffer to hold the input
        let mut buffer = String::new();
        
        // Wait for the user to press Enter
        io::stdin().read_line(&mut buffer).unwrap();
        
        info!("Key pressed. Continuing execution...");
        
        // Start client_sessions thread
        let client_sessions_handler = thread::spawn(move || {
            // Check every second for timeout clients
            loop {
                client_sessions.lock().unwrap().remove_timed_out_clients();
                thread::sleep(Duration::from_secs(1));
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
}
