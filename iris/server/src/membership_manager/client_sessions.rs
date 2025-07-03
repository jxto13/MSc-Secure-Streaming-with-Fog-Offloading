use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use dashmap::mapref::multiple::RefMulti;
use dashmap::DashMap;

use grid::Grid;
use iris_shared::bandwidth_utils::BandwidthManager;
use log::{debug, error, info};
use openssl::sha::sha256;
use openssl::x509::X509;
use proto::{ClientReport, MatrixReportBitmap};

use crate::metrics::ClientMetric;
use crate::reinforcement_learning::state::State;

use crate::{proto, CONFIG, DYNAMIC_VALUES};

#[derive(Debug)]
pub struct ClientData {
    pub client_id: Vec<u8>,  // Client ID using sha256 hash from public key
    pub last_seen: Instant,  // Last time the client was seen
    pub cert: X509,          // Client certificate
    pub ip: IpAddr,          // Client IP address
    pub report_counter: u32, // Counter for reports received from the client
    pub reports: Vec<MatrixReportBitmap>, // Matrix, Bitmap
    pub bandwidth_manager: BandwidthManager, // Bandwidth manager for the client
}

impl ClientData {
    fn find_zero_indices(vec: &Vec<u8>) -> Vec<usize> {
        vec.iter()
            .enumerate()
            .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
            .collect()
    }

    pub fn get_missing_packets(&self) -> Option<Vec<(u32, Vec<usize>)>> {
        if self.reports.is_empty() {
            return None;
        }

        let results: Vec<(u32, Vec<usize>)> = self
            .reports
            .iter()
            .map(|report| (report.matrix_index, Self::find_zero_indices(&report.bitmap)))
            .collect();

        Some(results)
    }
}

#[derive(Debug)]
pub struct ClientSessions {
    pub clients: DashMap<Vec<u8>, ClientData>, // ClientID, Client data
    pub reports_channel: Receiver<(SocketAddr, ClientReport)>, // Channel for receiving reports
}

impl ClientSessions {
    pub fn new(reports_channel: Receiver<(SocketAddr, ClientReport)>) -> Self {
        Self {
            clients: DashMap::new(), // Store client data
            reports_channel,
        }
    }

    /// Removes clients that have timed out and their associated reports.
    pub fn remove_timed_out_clients(&self) {
        let timeout = Duration::from_millis(CONFIG.get().unwrap().settings.timeout_clients);
        let now = Instant::now();
        let metrics = crate::metrics::get_metrics();

        // Collect keys of clients that have timed out
        let timed_out_keys: Vec<_> = self
            .clients
            .iter()
            .filter(|entry| now.duration_since(entry.value().last_seen) > timeout)
            .map(|entry| entry.key().clone())
            .collect();

        // Log the number of timed out clients
        for key in timed_out_keys {
            self.clients.remove(&key);

            let client_id_hex = hex::encode(&key);
            info!("Removing client: {:?}", client_id_hex); // Log the first 8 bytes of the client ID
            metrics.clear_client_metrics(&client_id_hex);
        }
    }

    pub fn start_manager(&self) {
        for (report_socket, report) in &self.reports_channel {
            // Get the client from the client_sessions
            match self.clients.get_mut(&report.client_id) {
                Some(mut client_data) => {
                    client_data.last_seen = Instant::now();
                    client_data.reports = report.reports;

                    if client_data.ip != report_socket.ip() {
                        client_data.ip = report_socket.ip();
                    }
                }
                None => {
                    error!(
                        "Received report from unknown client: {:?}",
                        report.client_id
                    );
                }
            }
        }
    }

    // Get the client cert from IpAddr
    pub fn get_client_cert(&self, client_id: &Vec<u8>) -> Option<X509> {
        // Get the client data for the given address
        if let Some(value) = self.clients.get(client_id) {
            Some(value.cert.clone())
        } else {
            None
        }
    }

    // Add client to the session
    pub fn add_client(&self, client_addr: IpAddr, cert: X509) {
        let client_id = sha256(&cert.public_key().unwrap().public_key_to_der().unwrap()).to_vec();

        // Create local data
        let client_data = ClientData {
            client_id: client_id.clone(),
            last_seen: Instant::now(),
            cert,
            ip: client_addr,
            report_counter: 0,
            reports: Vec::new(),
            bandwidth_manager: BandwidthManager::new(
                CONFIG.get().unwrap().retransmission.unicast_bitrate,
            ), // or hardcode bitrate here
        };

        // Create a hex representation of the client ID for metrics
        let client_id_hex = hex::encode(&client_id);

        // Insert the client in the session
        self.clients.insert(client_id, client_data);

        // Add the client to the metrics
        let metrics = crate::metrics::get_metrics();
        metrics.add_client(&client_id_hex);
    }

    /// Calculates overall packet loss across all clients.
    pub fn calculate_packet_loss(&self) -> f32 {
        let mut missing_packets = 0; // Deduplicated packets
        let mut total_expected_packets = 0; // Total packets expected

        // Iterate over all clients
        for client in &self.clients {
            for report in &client.reports {
                // Decode bitmap to get received packet indices
                let missing_packets_len = ClientData::find_zero_indices(&report.bitmap).len();

                // Add to the set of unique packets
                missing_packets += missing_packets_len;

                // Update the total expected packets
                total_expected_packets += report.bitmap.len();
            }
        }

        // Calculate packet loss percentage
        if total_expected_packets == 0 {
            return 0.0; // Avoid division by zero
        }

        // Calculate the percentage of received packets overall
        (missing_packets as f32 / total_expected_packets as f32) * 100.0
    }

    pub fn get_client_packet_loss(&self, client: &RefMulti<'_, Vec<u8>, ClientData>) -> f64 {
        let client_data = client.value();
        let mut missing_packets = 0; // Deduplicated packets
        let mut total_expected_packets = 0; // Total packets expected

        // Iterate over all reports for the client
        for report in &client_data.reports {
            // Decode bitmap to get received packet indices
            let missing_packets_len = ClientData::find_zero_indices(&report.bitmap).len();

            // Add to the set of unique packets
            missing_packets += missing_packets_len;

            // Update the total expected packets
            total_expected_packets += report.bitmap.len();
        }

        // Calculate packet loss percentage for this client
        if total_expected_packets == 0 {
            return 0.0; // Avoid division by zero
        }

        let packet_loss = (missing_packets as f64 / total_expected_packets as f64) * 100.0;

        packet_loss
    }

    pub fn get_session_state(&self) -> State {
        // Calculate the overall packet loss
        let packet_loss = self.calculate_packet_loss();

        State::from_loss(packet_loss)
    }

    pub fn get_client_unicast_rate(&self, client: &RefMulti<'_, Vec<u8>, ClientData>) -> f64 {
        let client_data = client.value();
        // Get the unicast rate from the bandwidth manager
        client_data.bandwidth_manager.get_rate_kbps() as f64
    }

    /// Updates the metrics for connected clients and their individual metrics.
    pub fn update_metrics(&self) {
        // Get the metrics instance
        let metrics = crate::metrics::get_metrics(); // get the Arc<Metrics>

        // Update global metrics
        {
            // Update connected clients metric
            metrics.connected_clients.set(self.clients.len() as i64);

            // Update FEC row and column metrics
            let dynamic_values_lock = DYNAMIC_VALUES.get().unwrap().read().unwrap();
            metrics
                .fec_row
                .set(dynamic_values_lock.fec_row_redundancy as i64);

            metrics
                .fec_col
                .set(dynamic_values_lock.fec_col_redundancy as i64);
        }

        // Set metric values for all clients
        for client in self.clients.iter() {
            // Get the client key (client ID)
            let client_id_bytes = client.key();

            // Convert client ID to hex string for metrics
            let client_id_hex = hex::encode(client_id_bytes);

            // Set unicast rate for client
            metrics
                .client_metric(ClientMetric::UnicastRate)
                .with_label_values(&[&client_id_hex])
                .set(self.get_client_unicast_rate(&client));

            // Set packet loss for client
            metrics
                .client_metric(ClientMetric::PacketLoss)
                .with_label_values(&[&client_id_hex])
                .set(self.get_client_packet_loss(&client));
        }
    }

    /// Generates the NC matrix and returns it along with the list of client addresses.
    pub fn generate_nc_matrix(&self) -> BTreeMap<u32, (Grid<u8>, Vec<IpAddr>)> {
        let mut nc_matrixes: BTreeMap<u32, (Grid<u8>, Vec<IpAddr>)> = BTreeMap::new();

        // this returns a vector with the matrix_index and its nc_matrix for all players, a snapshot of the current state

        for entry in &self.clients {
            let (_client_id, client_data) = entry.pair();
            for report in client_data.reports.iter() {
                let (matrix, clients_vec) = nc_matrixes
                    .entry(report.matrix_index)
                    .or_insert((Grid::new(0, 0), Vec::new()));
                matrix.push_row(report.bitmap.clone());
                clients_vec.push(client_data.ip.clone());
            }
        }

        nc_matrixes
    }
}
