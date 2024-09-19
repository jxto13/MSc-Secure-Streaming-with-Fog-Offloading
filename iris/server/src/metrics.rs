use once_cell::sync::OnceCell;
use prometheus_exporter::prometheus::{register_gauge, Gauge, opts};
use prometheus_exporter::prometheus::core::{AtomicF64, GenericGauge};
use std::sync::{Arc, Mutex};

// Struct to group related metrics
#[derive(Debug)]
pub struct Metrics {
    pub multicast_bandwidth_used: Mutex<GenericGauge<AtomicF64>>,  // Each gauge is wrapped in Mutex
    pub packets_received: Mutex<GenericGauge<AtomicF64>>,
    pub packets_sent: Mutex<GenericGauge<AtomicF64>>,
}

impl Metrics {
    // Initialize metrics
    pub fn new() -> Self {
        Metrics {
            multicast_bandwidth_used: Mutex::new(register_gauge!(opts!(
                "multicast_bandwidth_used", 
                "Total bandwidth used in bytes"
            )).unwrap()),
            
            packets_received: Mutex::new(register_gauge!(opts!(
                "packets_received_total", 
                "Total packets received"
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