use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::{Write, BufWriter};

use crossbeam_channel::{Receiver, Sender};
use iris::streaming_server::proto::{ClientReport, Empty};

use log::{info, error};
use serde::{Serialize, Deserialize};
use tonic::{Request, Response, Status};
use csv::WriterBuilder;

#[derive(Debug)]
pub struct ClientSessions {
    clients: Arc<Mutex<HashMap<SocketAddr, Instant>>>,
}

impl ClientSessions {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn update_client(&mut self, client_addr: SocketAddr) {
        let mut clients = self.clients.lock().unwrap();
        clients.insert(client_addr, Instant::now());
    }

    pub fn remove_timed_out_clients(&mut self) {
        let timeout = Duration::from_secs(5);
        let now = Instant::now();
        let mut clients = self.clients.lock().unwrap();
        clients.retain(|_, &mut last_seen| now.duration_since(last_seen) <= timeout);
        // info!("Clients after timeout removal: {:?}", *clients);
    }

    // Getter for the clients hashmap
    pub fn get_clients(&self) -> Arc<Mutex<HashMap<SocketAddr, Instant>>> {
        Arc::clone(&self.clients)
    }
}