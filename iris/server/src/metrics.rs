use once_cell::sync::OnceCell;
use prometheus_exporter::prometheus::{register_gauge, Gauge, opts};
use prometheus_exporter::prometheus::core::{AtomicF64, GenericGauge};
use std::sync::{Arc, Mutex};

// Struct to group related metrics
#[derive(Debug)]
pub struct Metrics {
    pub data_source_rate: Mutex<GenericGauge<AtomicF64>>,  // Each gauge is wrapped in Mutex
    pub multicast_sender_rate: Mutex<GenericGauge<AtomicF64>>,
    pub retransmissions_sender_rate: Mutex<GenericGauge<AtomicF64>>,
    pub packets_sent: Mutex<GenericGauge<AtomicF64>>,
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
            
            packets_sent: Mutex::new(register_gauge!(opts!(
                "packets_sent_total", 
                "Total packets sent"
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