use once_cell::sync::OnceCell;
use prometheus_exporter::prometheus::{opts, register_gauge, register_gauge_with_registry, register_int_gauge, Gauge, Registry};
use prometheus_exporter::prometheus::core::{AtomicF64, AtomicI64, GenericGauge, GenericGaugeVec};
use std::sync::{Arc, Mutex};

// Struct to group related metrics
#[derive(Debug)]
pub struct Metrics {
    pub data_source_rate: Mutex<GenericGauge<AtomicF64>>,  // Each gauge is wrapped in Mutex
    pub multicast_sender_rate: Mutex<GenericGauge<AtomicF64>>,
    pub retransmissions_sender_rate: Mutex<GenericGauge<AtomicF64>>,
    pub fec_row: Mutex<GenericGauge<AtomicI64>>,
    pub fec_col: Mutex<GenericGauge<AtomicI64>>,
    pub packet_loss: Mutex<GenericGauge<AtomicF64>>,

}

impl Metrics {
    // Initialize metrics
    pub fn new() -> Self {
        Metrics {
            data_source_rate: Mutex::new(register_gauge!(opts!(
                "data_source_rate", 
                "Rate of incoming UDP Stream"
            )).unwrap()),
            
            multicast_sender_rate: Mutex::new(register_gauge!(opts!(
                "multicast_sender_rate", 
                "Rate of what is being sent over multicast"
            )).unwrap()),

            retransmissions_sender_rate: Mutex::new(register_gauge!(opts!(
                "retransmissions_sender_rate", 
                "Rate of retransmissions being sent over multicast"
            )).unwrap()),
            
            fec_row: Mutex::new(register_int_gauge!(opts!(
                "fec_row", 
                "Number of rows for FEC"
            )).unwrap()),

            fec_col: Mutex::new(register_int_gauge!(opts!(
                "fec_col", 
                "Number of columns for FEC"
            )).unwrap()),

            packet_loss: Mutex::new(register_gauge!(opts!(
                "packet_loss", 
                "Packet loss rate"
            )).unwrap()),
        }
    }
}

// Global OnceCell to hold the Arc<Metrics> instance
static METRICS: OnceCell<Arc<Metrics>> = OnceCell::new();

// Function to initialize and set the metrics, if not already set
pub fn init_metrics() {
    let metrics = Arc::new(Metrics::new());
    METRICS.set(metrics).unwrap(); // Set the OnceCell with the Arc<Metrics>
}

// Function to access the metrics
pub fn get_metrics() -> Arc<Metrics> {
    METRICS.get().expect("Metrics not initialized").clone()
}