use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use log::info;
use proto::ClientReport;

use crate::proto;

#[derive(Debug)]
pub struct ClientData {
    last_seen: Instant,
    reports: HashMap<i32, Vec<u8>>, // Reports indexed by the report index
}

#[derive(Debug)]
pub struct ClientSessions {
    clients: Arc<Mutex<HashMap<SocketAddr, ClientData>>>,
}

impl ClientSessions {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Updates the client's last seen time or creates a new client entry.
    pub fn update_client(&mut self, client_addr: SocketAddr) {
        let mut clients = self.clients.lock().unwrap();
        clients
            .entry(client_addr)
            .and_modify(|client_data| client_data.last_seen = Instant::now()) // Update existing
            .or_insert(ClientData {
                last_seen: Instant::now(),
                reports: HashMap::new(),
            }); // Insert new
    }

    /// Removes clients that have timed out and their associated reports.
    pub fn remove_timed_out_clients(&mut self) {
        let timeout = Duration::from_secs(5);
        let now = Instant::now();
        let mut clients = self.clients.lock().unwrap();
        clients.retain(|_, client_data| now.duration_since(client_data.last_seen) <= timeout);
        // info!("Clients after timeout removal: {:?}", *clients);
    }

    /// Saves a client report, associating it with the client's session.
    pub fn save_client_report(&mut self, client_addr: SocketAddr, report: ClientReport) {
        // let mut clients = self.clients.lock().unwrap();
        // if let Some(client_data) = clients.get_mut(&client_addr) {
        //     client_data.reports.insert(report.index, report.payload.to_vec());
        // } else {
        //     // If the client doesn't exist, create a new entry
        //     let mut new_reports = HashMap::new();
        //     new_reports.insert(report.index, report.payload.to_vec());
        //     clients.insert(client_addr, ClientData {
        //         last_seen: Instant::now(),
        //         reports: new_reports,
        //     });
        // }
    }

    /// Optionally, retrieve client reports if needed
    pub fn get_client_reports(&self, client_addr: &SocketAddr) -> Option<HashMap<i32, Vec<u8>>> {
        let clients = self.clients.lock().unwrap();
        clients.get(client_addr).map(|data| data.reports.clone())
    }
}
