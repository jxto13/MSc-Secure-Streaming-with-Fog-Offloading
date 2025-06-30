// use once_cell::sync::OnceCell;
// use prometheus_exporter::prometheus::{opts, register_gauge, register_gauge_vec, register_int_gauge, IntGauge};
// use prometheus_exporter::prometheus::core::{AtomicF64, AtomicI64, GenericCounterVec, GenericGauge, GenericGaugeVec};
// use std::collections::HashMap;
// use std::sync::Arc;

// // Define client metrics using a macro for easier management
// macro_rules! client_metrics {
//     ( $( $name:ident ),* ) => {
//         #[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
//         pub enum ClientMetric {
//             $( $name ),*
//         }

//         impl ClientMetric {
//             pub fn iter() -> impl Iterator<Item = ClientMetric> {
//                 [ $( ClientMetric::$name ),* ].into_iter()
//             }
//         }
//     }
// }

// // Define client metrics
// client_metrics!(UnicastRate, PacketLoss);

// // Struct to group related metrics
// #[derive(Debug)]
// pub struct Metrics {
//     pub data_source_rate: GenericGauge<AtomicI64>,
//     pub multicast_sender_rate: GenericGauge<AtomicI64>,
//     pub retransmissions_sender_rate: GenericGauge<AtomicI64>,
//     pub unicast_sender_rate: GenericGauge<AtomicI64>,
//     pub fec_row: GenericGauge<AtomicI64>,
//     pub fec_col: GenericGauge<AtomicI64>,
//     pub connected_clients:  GenericGauge<AtomicI64>,
//     pub client_metrics: HashMap<ClientMetric, GenericGaugeVec<AtomicF64>>,
// }

// impl Metrics {
//     // Initialize metrics
//     pub fn new() -> Self {

//         let mut client_metrics = HashMap::new();

//         // Register metrics for each client metric type
//         for metric in ClientMetric::iter() {
//            let (name, help) = match metric {
//                 ClientMetric::UnicastRate => ("unicast_rate_client", "Unicast per client"),
//                 ClientMetric::PacketLoss => ("packet_loss_client", "Packet Loss per client"),
//             };

//             let gauge_vec = register_gauge_vec!(
//                 opts!(name, help),
//                 &["client_id"]
//             ).unwrap();

//             client_metrics.insert(metric, gauge_vec);
//         }


//         Metrics {
//             data_source_rate: register_int_gauge!(opts!(
//                 "data_source_rate", 
//                 "Rate of incoming UDP Stream"
//             )).unwrap(),
            
//             multicast_sender_rate: register_int_gauge!(opts!(
//                 "multicast_sender_rate", 
//                 "Rate of what is being sent over multicast"
//             )).unwrap(),

//             retransmissions_sender_rate: register_int_gauge!(opts!(
//                 "retransmissions_sender_rate", 
//                 "Rate of retransmissions being sent over multicast"
//             )).unwrap(),

//             unicast_sender_rate: register_int_gauge!(opts!(
//                 "unicast_sender_rate", 
//                 "Rate of what is being sent over unicast"
//             )).unwrap(),
            
//             fec_row: register_int_gauge!(opts!(
//                 "fec_row", 
//                 "Number of rows for FEC"
//             )).unwrap(),

//             fec_col: register_int_gauge!(opts!(
//                 "fec_col", 
//                 "Number of columns for FEC"
//             )).unwrap(),

//             connected_clients: register_int_gauge!(opts!(
//                 "connected_clients",
//                 "Total number of currently connected clients"
//             )).unwrap(),

//             client_metrics
//         }
//     }
// }

// // Global OnceCell to hold the Arc<Metrics> instance
// static METRICS: OnceCell<Arc<Metrics>> = OnceCell::new();

// // Function to initialize and set the metrics, if not already set
// pub fn init_metrics() {
//     let metrics = Arc::new(Metrics::new());
//     METRICS.set(metrics).unwrap(); // Set the OnceCell with the Arc<Metrics>
// }

// // Function to access the metrics
// pub fn get_metrics() -> Arc<Metrics> {
//     METRICS.get().expect("Metrics not initialized").clone()
// }


use once_cell::sync::OnceCell;
use prometheus_exporter::prometheus::{opts, register_gauge_vec, register_int_gauge};
use prometheus_exporter::prometheus::core::{AtomicF64, AtomicI64, GenericGaugeVec, GenericGauge};
use std::sync::Arc;

// Macro to define ClientMetric enum
macro_rules! client_metrics {
    ( $( $name:ident ),* ) => {
        #[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
        #[repr(usize)] // Ensure enum variants have stable usize discriminants
        pub enum ClientMetric {
            $( $name ),*
        }

        impl ClientMetric {
            pub fn iter() -> impl Iterator<Item = ClientMetric> {
                [ $( ClientMetric::$name ),* ].into_iter()
            }

            pub fn index(self) -> usize {
                self as usize
            }
        }
    }
}

// Define client metrics
client_metrics!(UnicastRate, PacketLoss);

// Struct to group related metrics
#[derive(Debug)]
pub struct Metrics {
    pub data_source_rate: GenericGauge<AtomicI64>,
    pub multicast_sender_rate: GenericGauge<AtomicI64>,
    pub retransmissions_sender_rate: GenericGauge<AtomicI64>,
    pub unicast_sender_rate: GenericGauge<AtomicI64>,
    pub fec_row: GenericGauge<AtomicI64>,
    pub fec_col: GenericGauge<AtomicI64>,
    pub connected_clients: GenericGauge<AtomicI64>,

    // Fixed-size vector of client metrics, each is a GaugeVec indexed by ClientMetric::index()
    pub client_metrics: Vec<GenericGaugeVec<AtomicF64>>,
}

impl Metrics {
    pub fn new() -> Self {
        let mut client_metrics = Vec::with_capacity(ClientMetric::iter().count());

        for metric in ClientMetric::iter() {
            let (name, help) = match metric {
                ClientMetric::UnicastRate => ("unicast_rate_client", "Unicast rate per client"),
                ClientMetric::PacketLoss => ("packet_loss_client", "Packet loss per client"),
            };

            let gauge_vec = register_gauge_vec!(
                opts!(name, help),
                &["client_id"]
            ).unwrap();

            client_metrics.push(gauge_vec);
        }

        Metrics {
            data_source_rate: register_int_gauge!(opts!(
                "data_source_rate", 
                "Rate of incoming UDP Stream"
            )).unwrap(),
            
            multicast_sender_rate: register_int_gauge!(opts!(
                "multicast_sender_rate", 
                "Rate of what is being sent over multicast"
            )).unwrap(),

            retransmissions_sender_rate: register_int_gauge!(opts!(
                "retransmissions_sender_rate", 
                "Rate of retransmissions being sent over multicast"
            )).unwrap(),

            unicast_sender_rate: register_int_gauge!(opts!(
                "unicast_sender_rate", 
                "Rate of what is being sent over unicast"
            )).unwrap(),
            
            fec_row: register_int_gauge!(opts!(
                "fec_row", 
                "Number of rows for FEC"
            )).unwrap(),

            fec_col: register_int_gauge!(opts!(
                "fec_col", 
                "Number of columns for FEC"
            )).unwrap(),

            connected_clients: register_int_gauge!(opts!(
                "connected_clients",
                "Total number of currently connected clients"
            )).unwrap(),

            client_metrics,
        }
    }

    // Helper to get client metric gauge vec by ClientMetric
    pub fn client_metric(&self, metric: ClientMetric) -> &GenericGaugeVec<AtomicF64> {
        &self.client_metrics[metric.index()]
    }

    // Add client to the metrics with a specific client_id
    pub fn add_client(&self, client_id: &str) {
        for gauge_vec in &self.client_metrics {
            // Add a new label value for the client_id
            // This initializes the metric for this client
            gauge_vec.with_label_values(&[client_id]);
        }
    }

    /// Remove all metrics for a given client_id across all client metric gauge vectors
    pub fn clear_client_metrics(&self, client_id: &str) {
        for gauge_vec in &self.client_metrics {
            // Remove all label values with client_id label
            // This clears metrics related to this client
            let _ = gauge_vec.remove_label_values(&[client_id]);
        }
    }
}

// Global OnceCell to hold the Arc<Metrics> instance
static METRICS: OnceCell<Arc<Metrics>> = OnceCell::new();

pub fn init_metrics() {
    let metrics = Arc::new(Metrics::new());
    METRICS.set(metrics).unwrap();
}

pub fn get_metrics() -> Arc<Metrics> {
    METRICS.get().expect("Metrics not initialized").clone()
}
