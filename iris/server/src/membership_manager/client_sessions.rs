use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use dashmap::DashMap;

use openssl::x509::X509;
use proto::{ClientReport, MatrixReportBitmap};

use crate::{proto, CONFIG};

#[derive(Debug)]
pub struct ClientData {
    pub last_seen: Instant,               // Last time the client was seen
    pub cert: X509,                       // Client certificate
    pub reports: Vec<MatrixReportBitmap>, // Matrix, Bitmap
}

impl ClientData {
    fn find_zero_indices(vec: &Vec<u8>) -> Vec<usize> {
        vec.iter()
            .enumerate()
            .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
            .collect()
    }

    pub fn get_missing_packets(&self) -> Option<Vec<(u32,Vec<usize>)>> {
        if self.reports.is_empty() {
            return None;
        }
    
        let results: Vec<(u32, Vec<usize>)> = self.reports.iter()
            .map(|report| (report.matrix_index, Self::find_zero_indices(&report.bitmap)))
            .collect();
    
        Some(results)
    }
}

#[derive(Debug)]
pub struct ClientSessions {
    pub clients: DashMap<IpAddr, ClientData>, // Client address, Client data
    pub reports_channel: Receiver<(SocketAddr, ClientReport)>, // Channel for receiving reports
}

impl ClientSessions {
    pub fn new(reports_channel: Receiver<(SocketAddr, ClientReport)>) -> Self {
        Self {
            clients: DashMap::new(), // Store client data
            reports_channel
        }
    }

    /// Removes clients that have timed out and their associated reports.
    pub fn remove_timed_out_clients(&self) {
        let timeout = Duration::from_secs(CONFIG.get().unwrap().settings.timeout_clients);
        let now = Instant::now();
        self.clients
            .retain(|_, client_data| now.duration_since(client_data.last_seen) <= timeout);
    }

    pub fn start_manager(&self) {

        for (client_addr,report) in &self.reports_channel {

            // info!("Received report: {:?}", report);

            match self.clients.get_mut(&client_addr.ip()){
                Some(mut client_data) => {
                    // Update the last seen time
                    client_data.last_seen = Instant::now();
                    // Add the report to the client data

                    client_data.reports = report.reports;
                }
                None => {
                    // If the client is not found, create a new entry
                    // let cert = self.get_client_cert(client_addr);
                    // if let Some(cert) = cert {
                    //     self.add_client(client_addr, cert);
                    //     self.clients.get_mut(client_addr).unwrap().reports.push((report.matrix_index, report.bitmap));
                    // } else {
                    //     info!("Client certificate not found for address: {:?}", client_addr);
                    // }
                }
            }
            
        }


    }

    // Get the client cert from IpAddr
    pub fn get_client_cert(&self, client_addr: IpAddr) -> Option<X509> {
        // Get the client data for the given address
        if let Some(value) = self.clients.get(&client_addr) {
            Some(value.cert.clone())
        } else {
            None
        }
    }

    // Add client to the session
    pub fn add_client(&self, client_addr: IpAddr, cert: X509) {
        // Create local data
        let client_data = ClientData {
            last_seen: Instant::now(),
            cert,
            reports: Vec::new(),
        };

        // Insert the client in the session
        self.clients.insert(client_addr, client_data);

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
        for client in &self.clients {
            for report in &client.reports {
                // Decode bitmap to get received packet indices
                let received_packets_len = Self::find_zero_indices(&report.bitmap).len();

                // Add to the set of unique packets
                received_packets += received_packets_len;

                // Update the total expected packets
                total_expected_packets += report.bitmap.len();
            }
        }

        // Calculate packet loss percentage
        if total_expected_packets == 0 {
            return -1.0; // Avoid division by zero
        }

        // Calculate the percentage of received packets overall
        (received_packets as f32 / total_expected_packets as f32) * 100.0
    }

    

    // pub fn add_client(&self, client_addr: IpAddr, cert: X509) {
    //     // Add a new client with the given address and certificate
    //     self.clients.insert(
    //         client_addr,
    //         ClientData {
    //             last_seen: Instant::now(),
    //             cert,
    //             reports: Vec::new(),
    //         },
    //     );
    // }

    // pub fn get_client_cert(&self, client_addr: IpAddr) -> Option<X509> {
    //     // Get the certificate for the given client address
    //     self.clients.get(&client_addr).map(|client_data| client_data.cert.clone())
    // }

    // /// Updates the client's last seen time or creates a new client entry.
    // pub fn update_client(&mut self, client_addr: IpAddr) -> &mut ClientData {
    //     self.clients
    //         .entry(client_addr)
    //         .and_modify(|client_data| client_data.last_seen = Instant::now()) // Update existing
    //         // Insert new
    //         .or_insert(ClientData {
    //             last_seen: Instant::now(),
    //             reports: Vec::new(),
    //         })
    // }

    

    // /// Saves a client report, associating it with the client's session.
    // pub fn save_client_report(&mut self, client_addr: IpAddr, report: ClientReport) {
    //     // Update the last seen time for the client
    //     let client_entry = self.update_client(client_addr);

    //     // Clear the reports for the client
    //     client_entry.reports.clear();

    //     // Add all reports by matrix as key in hash
    //     for item in report.reports {
    //         client_entry.reports.push((item.matrix_index, item.bitmap));
    //     }
    // }

    // fn find_zero_indices(vec: &Vec<u8>) -> Vec<usize> {
    //     vec.iter()
    //         .enumerate()
    //         .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
    //         .collect()
    // }

    // /// Calculates overall packet loss across all clients.
    // pub fn calculate_packet_loss(&self) -> f32 {
    //     let mut received_packets = 0; // Deduplicated packets
    //     let mut total_expected_packets = 0; // Total packets expected

    //     // Iterate over all clients
    //     for client_data in self.clients.values() {
    //         for (_, bitmap) in &client_data.reports {
    //             // Decode bitmap to get received packet indices
    //             let received_packets_len = Self::find_zero_indices(&bitmap.to_vec()).len();

    //             // Add to the set of unique packets
    //             received_packets += received_packets_len;

    //             // Update the total expected packets
    //             total_expected_packets += bitmap.len();
    //         }
    //     }

    //     // Calculate packet loss percentage
    //     if total_expected_packets == 0 {
    //         return -1.0; // Avoid division by zero
    //     }

    //     // Calculate the percentage of received packets overall
    //     (received_packets as f32 / total_expected_packets as f32) * 100.0
    // }

    // /// Generates the NC matrix and returns it along with the list of client addresses.
    // pub fn generate_nc_matrix(&self) -> BTreeMap<i32, (Grid<u8>, Vec<IpAddr>)> {
    //     let mut nc_matrixes: BTreeMap<i32, (Grid<u8>, Vec<IpAddr>)> = BTreeMap::new();

    //     // this returns a vector with the matrix_index and its nc_matrix for all players, a snapshot of the current state

    //     for (client_addr, client_data) in self.clients.iter() {
    //         for (matrix_index, bitmap) in client_data.reports.iter() {
    //             let (matrix, clients_vec) = nc_matrixes
    //                 .entry(*matrix_index)
    //                 .or_insert((Grid::new(0, 0), Vec::new()));
    //             matrix.push_row(bitmap.clone());
    //             clients_vec.push(client_addr.clone());
    //         }
    //     }

    //     nc_matrixes
    // }
}
