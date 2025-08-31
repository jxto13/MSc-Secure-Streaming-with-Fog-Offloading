use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use dashmap::mapref::multiple::RefMulti;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;

use grid::Grid;
use iris_shared::bandwidth_utils::{
    BandwidthManager, ClientBandwidthLimiter, MulticastBandwidthManager,
};
use log::{error, info};
use openssl::sha::sha256;
use openssl::x509::X509;
use proto::{ClientReport, MatrixReportBitmap};

use crate::metrics::ClientMetric;
use crate::proto::ReportMetrics;
use crate::reinforcement_learning::state::State;

use crate::{proto, CONFIG, DYNAMIC_VALUES};

pub fn find_zero_indices(vec: &Vec<u8>) -> Vec<usize> {
    vec.iter()
        .enumerate()
        .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
        .collect()
}

#[derive(Debug)]
pub struct ClientData {
    pub client_id: Vec<u8>,     // Client ID using sha256 hash from public key
    pub last_seen: Instant,     // Last time the client was seen
    pub cert: X509,             // Client certificate
    pub ip: IpAddr,             // Client IP address
    pub metrics: ReportMetrics, // Client metrics
    pub report_counter: u32,    // Counter for reports received from the client
    pub reports: Vec<MatrixReportBitmap>, // Matrix, Bitmap
    pub latency_estimate: Duration, // Estimated one-way latency to this client
    pub last_report_time: Option<Instant>, // Timestamp of last report for latency calculation
}

impl ClientData {
    pub fn get_missing_packets(&self, matrix_id: u32) -> Option<Vec<usize>> {
        for report in &self.reports {
            if report.matrix_index == matrix_id {
                return Some(find_zero_indices(&report.bitmap));
            }
        }
        None
    }
}

#[derive(Debug)]
pub struct ClientSessions {
    pub clients: DashMap<Vec<u8>, ClientData>, // ClientID, Client data
    pub client_bandwidth_manager: ClientBandwidthLimiter, // Bandwidth manager for clients
    pub mutlicast_bandwidth_manager: MulticastBandwidthManager, // Bandwidth manager for multicast
    pub reports_channel: Receiver<(SocketAddr, ClientReport)>, // Channel for receiving reports
}

impl ClientSessions {
    pub fn new(reports_channel: Receiver<(SocketAddr, ClientReport)>) -> Self {
        Self {
            clients: DashMap::new(), // Store client data
            client_bandwidth_manager: ClientBandwidthLimiter::new(
                CONFIG.get().unwrap().retransmission.unicast_bitrate,
            ),
            mutlicast_bandwidth_manager: MulticastBandwidthManager::new(
                CONFIG.get().unwrap().multicast.bitrate,
            ),
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
                    let now = Instant::now();
                    client_data.last_seen = now;
                    client_data.reports = report.reports;
                    client_data.metrics = report.metrics.unwrap();

                    // Update latency estimate based on report timing patterns
                    if let Some(last_report_time) = client_data.last_report_time {
                        let config = CONFIG.get().unwrap();
                        let expected_interval =
                            Duration::from_millis(config.settings.client_reporting_interval as u64);
                        let actual_interval = now.duration_since(last_report_time);

                        // Estimate one-way latency as half the excess delay
                        // This assumes symmetric network paths
                        if actual_interval > expected_interval {
                            let excess_delay = actual_interval - expected_interval;
                            let measured_latency = excess_delay / 2; // Assume half is one-way latency

                            // Use exponential moving average to smooth latency estimates
                            // New latency = 0.2 * measured + 0.8 * old_estimate (faster adaptation than RTT)
                            client_data.latency_estimate = Duration::from_millis(
                                ((measured_latency.as_millis() as f64 * 0.2)
                                    + (client_data.latency_estimate.as_millis() as f64 * 0.8))
                                    as u64,
                            );

                            // log::debug!(
                            //     "Updated latency estimate for client {:?}: {}ms",
                            //     hex::encode(&report.client_id[..4]), // First 4 bytes for readability
                            //     client_data.latency_estimate.as_millis()
                            // );
                        } else {
                            // If reports arrive early or on time, gradually reduce latency estimate
                            // This handles cases where network conditions improve
                            let min_latency = Duration::from_millis(5); // Minimum realistic latency
                            if client_data.latency_estimate > min_latency {
                                client_data.latency_estimate = Duration::from_millis(
                                    (client_data.latency_estimate.as_millis() as f64 * 0.95) as u64,
                                )
                                .max(min_latency);
                            }
                        }
                    }

                    // Update the last report time for next RTT calculation
                    client_data.last_report_time = Some(now);

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
            metrics: ReportMetrics::default(),
            report_counter: 0,
            reports: Vec::new(),
            latency_estimate: Duration::from_millis(25), // Initial one-way latency estimate for WiFi
            last_report_time: None,
        };

        // Create a hex representation of the client ID for metrics
        let client_id_hex = hex::encode(&client_id);

        // Insert the client in the session
        self.clients.insert(client_id, client_data);

        // Add the client to the metrics
        let metrics = crate::metrics::get_metrics();
        metrics.add_client(&client_id_hex);
    }

    // Get client data by client ID
    pub fn get_client_data(&self, client_id: &Vec<u8>) -> Option<Ref<'_, Vec<u8>, ClientData>> {
        self.clients.get(client_id)
    }

    /// Calculates overall packet loss across all clients.
    pub fn calculate_packet_loss(&self) -> f32 {
        let mut missing_packets = 0; // Deduplicated packets
        let mut total_expected_packets = 0; // Total packets expected

        // Iterate over all clients
        for client in &self.clients {
            for report in &client.reports {
                // Decode bitmap to get received packet indices
                let missing_packets_len = find_zero_indices(&report.bitmap).len();

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

    pub fn get_missed_deadlines(&self, client: &RefMulti<'_, Vec<u8>, ClientData>) -> usize {
        let mut total_missed_deadlines = 0;

        for report in &client.value().reports {
            total_missed_deadlines += find_zero_indices(&report.bitmap).len();
        }

        total_missed_deadlines
    }

    pub fn get_client_packet_loss(&self, client: &RefMulti<'_, Vec<u8>, ClientData>) -> f64 {
        let client_data = client.value();
        let mut missing_packets = 0; // Deduplicated packets
        let mut total_expected_packets = 0; // Total packets expected

        // Iterate over all reports for the client
        for report in &client_data.reports {
            // Decode bitmap to get received packet indices
            let missing_packets_len = find_zero_indices(&report.bitmap).len();

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
        self.client_bandwidth_manager
            .get_rate_kbps(&client_data.client_id)
            .unwrap_or(0) as f64
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

            // Set missed deadlines for client
            metrics
                .client_metric(ClientMetric::MissedDeadlines)
                .with_label_values(&[&client_id_hex])
                .set(self.get_missed_deadlines(&client) as f64);

            // Set packet loss for client
            metrics
                .client_metric(ClientMetric::PacketLoss)
                .with_label_values(&[&client_id_hex])
                .set(self.get_client_packet_loss(&client));

            // Set latency estimate for client
            metrics
                .client_metric(ClientMetric::LatencyEstimate)
                .with_label_values(&[&client_id_hex])
                .set(client.value().latency_estimate.as_millis() as f64);

            // Set counter for reports received from the client
            metrics
                .client_metric(ClientMetric::MulticastRetransmissions)
                .with_label_values(&[&client_id_hex])
                .set(client.value().metrics.multicast_retransmissions as f64);

            metrics
                .client_metric(ClientMetric::UnicastRetransmissions)
                .with_label_values(&[&client_id_hex])
                .set(client.value().metrics.unicast_retransmissions as f64);

            metrics
                .client_metric(ClientMetric::WifiAwareRetransmissions)
                .with_label_values(&[&client_id_hex])
                .set(client.value().metrics.wifi_aware_retransmissions as f64);

            // Update the tick bandwidth manager for the client
            self.client_bandwidth_manager.tick();
        }
    }

    /// Generates the NC matrix and returns it along with the list of client addresses.
    pub fn generate_nc_matrix(&self) -> BTreeMap<u32, (Grid<u8>, Vec<(Vec<u8>, IpAddr)>)> {
        let mut nc_matrixes: BTreeMap<u32, (Grid<u8>, Vec<(Vec<u8>, IpAddr)>)> = BTreeMap::new();

        // this returns a vector with the matrix_index and its nc_matrix for all players, a snapshot of the current state

        for entry in &self.clients {
            let (client_id, client_data) = entry.pair();
            for report in client_data.reports.iter() {
                let (matrix, clients_vec) = nc_matrixes
                    .entry(report.matrix_index)
                    .or_insert((Grid::new(0, 0), Vec::new()));
                matrix.push_row(report.bitmap.clone());
                clients_vec.push((client_id.clone(), client_data.ip.clone()));
            }
        }

        nc_matrixes
    }
}
