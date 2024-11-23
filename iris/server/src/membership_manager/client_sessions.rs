use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use proto::ClientReport;

use crate::{proto, CONFIG};

#[derive(Debug)]
pub struct ClientData {
    last_seen: Instant,
    pub reports: BTreeMap<i32, Vec<u8>>, // Reports indexed by the report index
}

#[derive(Debug)]
pub struct ClientSessions {
    pub clients: HashMap<SocketAddr, ClientData>,
}

impl ClientSessions {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Updates the client's last seen time or creates a new client entry.
    pub fn update_client(&mut self, client_addr: SocketAddr) {

        self.clients
            .entry(client_addr)
            .and_modify(|client_data| client_data.last_seen = Instant::now()) // Update existing
            // Insert new
            .or_insert(ClientData {
                last_seen: Instant::now(),
                reports: BTreeMap::new(),
            });
    }

    /// Removes clients that have timed out and their associated reports.
    pub fn remove_timed_out_clients(&mut self) {
        let timeout = Duration::from_secs(CONFIG.get().unwrap().settings.timeout_clients);
        let now = Instant::now();
        self.clients.retain(|_, client_data| now.duration_since(client_data.last_seen) <= timeout);
    }

    /// Saves a client report, associating it with the client's session.
    pub fn save_client_report(&mut self, client_addr: SocketAddr, report: ClientReport) {

        if let Some(client_data) = self.clients.get_mut(&client_addr) {
            // Add all reports by matrix as key in hash
            for item in report.reports {
                client_data.reports.insert(item.matrix, item.bitmap);
            }
        } else {
            // If the client doesn't exist, create a new entry
            let mut new_reports = BTreeMap::new();
            for item in report.reports {
                new_reports.insert(item.matrix, item.bitmap);
            }
            self.clients.insert(client_addr, ClientData {
                last_seen: Instant::now(),
                reports: new_reports,
            });
        }
    }

}
