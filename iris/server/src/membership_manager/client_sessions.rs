use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use iris_shared::ConcurrentVector;
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
    pub clients: ConcurrentVector<SocketAddr,  Arc<RwLock<ClientData>>>, // Client address, Client data
}

impl ClientSessions {
    pub fn new() -> Self {
        Self {
            clients: ConcurrentVector::new(),
        }
    }
    

    /// Removes clients that have timed out and their associated reports.
    pub fn remove_timed_out_clients(&self) {
        let timeout = Duration::from_secs(CONFIG.get().unwrap().settings.timeout_clients);
        let now = Instant::now();
        // self.clients.retain(|_, client_data| now.duration_since(client_data.write().unwrap().last_seen) <= timeout);
    }

    /// Saves a client report, associating it with the client's session.
    pub fn save_client_report(&mut self, client_addr: SocketAddr, report: ClientReport) {

        // Check if the client already exists in the map
        let client_entry = self.clients.get(&client_addr);

        // If the client doesn't exist, create a new entry
        if client_entry.is_none() {

            // Create a new client data entry
            let mut new_client_data = ClientData {
                last_seen: Instant::now(),
                reports: Vec::new(),
            };

            // Add all reports by matrix as key in hash
            for item in report.reports {
                new_client_data.reports.push((item.matrix_index, item.bitmap));
            }
            
            // Create a new Arc of the client data
            let new_client_entry = Arc::new(RwLock::new(new_client_data));
            
            // Insert the new client data into the map
            self.clients.insert(client_addr, new_client_entry)
            
        } else {
            // If the client exists, get the Arc of client
            let client_arc = client_entry.unwrap();

            // Update last seen time safely
            client_arc.write().unwrap().last_seen = Instant::now();

            // Lock the client data for writing
            let mut client_data = client_arc.write().unwrap();

            // Clear the reports for the client
            client_data.reports.clear();

            // Add all reports by matrix as key in hash
            for item in report.reports {
                client_data.reports.push((item.matrix_index, item.bitmap));
            }

        }
        
    }

    /// Finds the indices of zero values in a vector.
    fn find_zero_indices(vec: Vec<u8>) -> Vec<usize> {
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

            // Lock the client data for reading
            let clientdata = client_data.read().unwrap();

            for (_, bitmap) in &clientdata.reports {
                // Decode bitmap to get received packet indices
                let received_packets_len = Self::find_zero_indices(bitmap.to_vec()).len();

                // Add to the set of unique packets
                received_packets += received_packets_len;

                // Update the total expected packets
                total_expected_packets += bitmap.len(); 
            }
        }

        // Calculate packet loss percentage
        if total_expected_packets == 0 {
            return 0.0; // Avoid division by zero
        }

        // Calculate the percentage of received packets overall
        (received_packets as f32 / total_expected_packets as f32) * 100.0
    }

    /// Decodes a bitmap into the set of received packet indices.
    fn decode_bitmap(matrix_index: i32, bitmap: &[u8]) -> HashSet<i32> {
        let mut received_packets = HashSet::new();

        for (byte_index, byte) in bitmap.iter().enumerate() {
            for bit_index in 0..8 {
                if (byte >> bit_index) & 1 == 1 {
                    received_packets.insert(matrix_index + (byte_index * 8 + bit_index) as i32);
                }
            }
        }

        received_packets
    }

    /// Generates the NC matrix and returns it along with the list of client addresses.
    pub fn generate_nc_matrix(&self) -> BTreeMap<i32, (Grid<u8>, Vec<SocketAddr>)> {
        let mut nc_matrixes: BTreeMap<i32, (Grid<u8>, Vec<SocketAddr>)> = BTreeMap::new();

        // this returns a vector with the matrix_index and its nc_matrix for all players, a snapshot of the current state
    
        for (client_addr, client_data) in self.clients.iter() {
            // Lock the client data for reading
            let client_data = client_data.read().unwrap();

            for (matrix_index, bitmap) in client_data.read().unwrap().reports.iter() {
                let (matrix, clients_vec) = nc_matrixes.entry(*matrix_index).or_insert((Grid::new(0, 0), Vec::new()));
                matrix.push_row(bitmap.clone());
                clients_vec.push(client_addr.clone());
            }
        }

        nc_matrixes
        
    }

}
