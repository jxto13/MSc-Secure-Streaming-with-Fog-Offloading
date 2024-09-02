use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use proto::ClientReport;

use crate::proto;

#[derive(Debug)]
pub struct ClientSessions {
    clients: Arc<Mutex<HashMap<SocketAddr, Instant>>>,
    reports: Arc<Mutex<HashMap<SocketAddr, HashMap<i32, Vec<u8>>>>>, // To store reports per client
}

impl ClientSessions {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            reports: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn save_client_report(&mut self, client_addr: SocketAddr, report: ClientReport) {
        let mut reports = self.reports.lock().unwrap();
        let client_reports = reports.entry(client_addr).or_insert_with(HashMap::new);

        client_reports.insert(report.index, report.payload.to_vec());
    }
}