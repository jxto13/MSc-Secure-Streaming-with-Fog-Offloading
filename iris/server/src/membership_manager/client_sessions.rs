use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use openssl::x509::X509;
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
    pub certs: HashMap<IpAddr, X509>, // Client address, Client certificate
    pub clients: HashMap<IpAddr, ClientData>, // Client address, Client data
}

impl ClientSessions {
    pub fn new() -> Self {
        Self {
            certs: HashMap::new(), // Store client certificates
            clients: HashMap::new(), // Store client data
        }
    }

    /// Updates the client's last seen time or creates a new client entry.
    pub fn update_client(&mut self, client_addr: IpAddr) -> &mut ClientData {

        self.clients
            .entry(client_addr)
            .and_modify(|client_data| client_data.last_seen = Instant::now()) // Update existing
            // Insert new
            .or_insert(ClientData {
                last_seen: Instant::now(),
                reports: Vec::new(),
            })
    }

    /// Removes clients that have timed out and their associated reports.
    pub fn remove_timed_out_clients(&mut self) {
        let timeout = Duration::from_secs(CONFIG.get().unwrap().settings.timeout_clients);
        let now = Instant::now();
        self.clients.retain(|_, client_data| now.duration_since(client_data.last_seen) <= timeout);
    }

    /// Saves a client report, associating it with the client's session.
    pub fn save_client_report(&mut self, client_addr: IpAddr, report: ClientReport) {

        // Update the last seen time for the client
        let client_entry = self.update_client(client_addr);

        // Clear the reports for the client
        client_entry.reports.clear();

        // Add all reports by matrix as key in hash
        for item in report.reports {
            client_entry.reports.push((item.matrix_index, item.bitmap));
        }

    }

    fn find_zero_indices(vec: &Vec<u8>) -> Vec<usize> {
        vec.iter()
            .enumerate()
            .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
            .collect()
    }

    /// Calculates overall packet loss across all clients.
    pub fn calculate_packet_loss(&self) -> f32 {
        let mut received_packets = 0; // Deduplicated packets
        let mut total_expected_packets = 0; // Total packets expected

        // Iterate over all clients
        for client_data in self.clients.values() {
            for (_, bitmap) in &client_data.reports {
                // Decode bitmap to get received packet indices
                let received_packets_len = Self::find_zero_indices(&bitmap.to_vec()).len();

                // Add to the set of unique packets
                received_packets += received_packets_len;

                // Update the total expected packets
                total_expected_packets += bitmap.len(); 
            }
        }

        // Calculate packet loss percentage
        if total_expected_packets == 0 {
            return -1.0; // Avoid division by zero
        }

        // Calculate the percentage of received packets overall
        (received_packets as f32 / total_expected_packets as f32) * 100.0
    }

    /// Generates the NC matrix and returns it along with the list of client addresses.
    pub fn generate_nc_matrix(&self) -> BTreeMap<i32, (Grid<u8>, Vec<IpAddr>)> {
        let mut nc_matrixes: BTreeMap<i32, (Grid<u8>, Vec<IpAddr>)> = BTreeMap::new();

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
