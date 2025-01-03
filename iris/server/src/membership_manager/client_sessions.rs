use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use proto::ClientReport;
use grid::Grid;

use crate::{proto, CONFIG};

#[derive(Debug)]
pub struct ClientData {
    last_seen: Instant, // Last time the client was seen
    pub reports: Vec<(i32, Vec<u8>)>, // Matrix, Bitmap
}

#[derive(Debug)]
pub struct ClientSessions {
    pub clients: HashMap<SocketAddr, ClientData>, // Client address, Client data
    pub runtime: Arc<tokio::runtime::Runtime>,
}

impl ClientSessions {
    pub fn new(rt: Arc<tokio::runtime::Runtime>) -> Self {
        Self {
            clients: HashMap::new(),
            runtime: rt,
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
                reports: Vec::new(),
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
            // Clear the reports for the client
            client_data.reports.clear();

            // Add all reports by matrix as key in hash
            for item in report.reports {
                client_data.reports.push((item.matrix, item.bitmap));
            }
        } else {
            // If the client doesn't exist, create a new entry
            let mut new_reports = Vec::new();

            // Add all reports by matrix 
            for item in report.reports {
                new_reports.push((item.matrix, item.bitmap));
            }

            // Insert the new client
            self.clients.insert(client_addr, ClientData {
                last_seen: Instant::now(),
                reports: new_reports,
            });
        }
    }

    /// Generates the NC matrix and returns it along with the list of client addresses.
    pub fn generate_nc_matrix(&self) -> BTreeMap<i32, (Grid<u8>, Vec<SocketAddr>)> {
        let mut nc_matrixes: BTreeMap<i32, (Grid<u8>, Vec<SocketAddr>)> = BTreeMap::new();

        // this returns a vector with the matrix_index and its nc_matrix for all players, a snapshot of the current state
    
        for (client_addr, client_data) in self.clients.iter() {
            for (matrix_index, bitmap) in client_data.reports.iter() {
                let (matrix, clients_vec) = nc_matrixes.entry(*matrix_index).or_insert((Grid::new(0, 0), Vec::new()));
                matrix.push_row(bitmap.clone());
                clients_vec.push(client_addr.clone());
            }
        }

        nc_matrixes
        
    }

}
