mod crypto;
mod network_scheduler;
mod source_handler;
mod membership_manager;
mod metrics;
mod nts_service;

use std::sync::RwLock;
use std::time::Duration;
use std::{sync::Arc, thread};
use std::path::PathBuf;

// Standard
use crossbeam_channel::unbounded;
use crypto::key_manager::KeyManager;
use grid::Grid;
use log::{error, info};
use membership_manager::client_sessions;
use membership_manager::reporting_sink::MyReportService;
use crypto::certificates;
use once_cell::sync::OnceCell;
use openssl::x509::X509;
use proto::nts_service_server::NtsServiceServer;
use source_handler::data_handler::{DataHandler,DataSource};

use tonic::transport::{Identity, Server, ServerTlsConfig};
use tokio::runtime::Runtime;

use proto::TransmissionPacket;

use iris_server::{read_config,Config};

use client_sessions::ClientSessions;
use crate::nts_service::MyNtsService;
use crate::metrics::init_metrics;


// Import protobuff
pub mod proto {
    include!("../proto/generated/streaming_server.rs");
}

// Define the global static instance of Config using OnceCell
static CONFIG: OnceCell<Config> = OnceCell::new();
static MANAGER: OnceCell<Arc<RwLock<KeyManager>>> = OnceCell::new();
static MATRIX_BUFFER: OnceCell<Arc<MatrixHistory>> = OnceCell::new();

// Custom
use network_scheduler::pipeline::Pipeline;
use clap::{arg, Command};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set a custom panic hook to log panics and exit gracefully
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("Panic occurred: {:?}", panic_info);
        std::process::exit(1);
    }));

    // Define the command structure
    let cmd = Command::new("streaming_server")
        .version("0.1.0")
        .author("Joaquim Oliveira <up201908075@up.pt>")
        .about("MSc Secure Streaming with Fog Offloading")
        .arg(
            arg!(--"config" <FILE>)
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to the configuration file"),
        )
        .arg(
            arg!(--"train")
                .required(false)
                .help("Enable training mode for RL simulation")
                .action(clap::ArgAction::SetTrue),
        );
    
    // Parse command-line arguments
    let matches = cmd.get_matches();
    let config_filename = matches.get_one::<PathBuf>("config").expect("required argument");

    // Init the logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()
        .init();

    // tracing_subscriber::fmt()
    //     // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
    //     // will be written to stdout.
    //     .with_max_level(tracing::Level::TRACE)
    //     // sets this to be the default, global collector for this application.
    //     .init();

    if matches.get_flag("train") {
        info!("------------------------- Training mode enabled  -------------------------");
    } else {
        info!("------------------------- Training mode disabled -------------------------");
    }

    // Read configuration file
    let config = read_config(config_filename.to_str().unwrap())?;

    // Set the global CONFIG instance
    CONFIG.set(config).expect("Failed to set CONFIG");

    MATRIX_BUFFER.set(Arc::new(MatrixHistory::new(64))).expect("Failed to set MATRIX_BUFFER");
    
    // Start the Prometheus exporter
    start_prometheus_exporter();

    // Set up a Tokio runtime
    let rt = Arc::new(Runtime::new().unwrap());
    
    // ----------------------------------- Programs Setup -------------------------------------------------------------

    // Load or create certificates
    let certificates = certificates::load_or_create_certificates().unwrap();
    let key_manager = Arc::new(RwLock::new(KeyManager::new(certificates.ec_key)));

    // Start key_service before the pipeline to generate the ECDSA Keys for encoder 
    let key_service = crypto::key_service::get_service(Arc::clone(&key_manager), X509::from_pem(certificates.ec_cert.as_bytes()).unwrap());

    MANAGER.set(key_manager).unwrap();

    // ----------------------------------- Programs Starts Here ------------------------------------------------------

    // Create channels
    let (source_handler_sender, source_handler_receiver) = unbounded();
    let (reports_channel_sender, reports_channel_receiver) = unbounded();
    
    // Choose the data source
    let source = match CONFIG.get().unwrap().multicast.data_source.as_str() {
        "udp" => DataSource::UDP(CONFIG.get().unwrap().multicast.ip_source.clone()),
        _ => panic!("Invalid data_source value in config"),
    };
    
    let source_handler = thread::spawn(move || {
        // Create a DataHandler instance based on datasource
        DataHandler::new(source, source_handler_sender).start();
    });
    

    // Create the client sessions
    let client_sessions = Arc::new(ClientSessions::new(reports_channel_receiver));

    // Start client_sessions thread
    let client_sessions_handler = thread::spawn({
        let client_sessions = Arc::clone(&client_sessions);
        move || {
            client_sessions.start_manager();
        }
    });

    // Get the retransmission config
    let retransmission_config = &CONFIG.get().unwrap().retransmission;
    
    // Create the pipeline
    let pipeline= Arc::new(Pipeline::new(Arc::clone(&client_sessions),retransmission_config.wifi_multicast, retransmission_config.wifi_unicast));

    let pipeline_handler = {
        let pipeline = Arc::clone(&pipeline);
        thread::spawn(move || {
            pipeline.start(source_handler_receiver);
        })
    };

    // Creates the reporting sink
    MyReportService::new(Arc::clone(&client_sessions), reports_channel_sender);

    // Create a std thread and setup tokio for gRPC Server with tokio runtime
    let grpc_server_handler = thread::spawn({
        let rt = Arc::clone(&rt);
        let client_sessions = Arc::clone(&client_sessions);
        move || {
        // Run async code using the Tokio runtime
        rt.block_on(async {

            // listen to all ipv4 addresses
            let addr = format!("0.0.0.0:{}", CONFIG.get().unwrap().grpc.port).parse().unwrap();
            

            // Start grpc server this will block execution because of await
            Server::builder()
                .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(certificates.server_cert, certificates.server_key))).unwrap()
                .add_service(crypto::ca_service::get_service(certificates.ca_cert, certificates.ca_private_key, client_sessions)) // Responsible for signing the client certificates
                .add_service(key_service)
                .add_service(NtsServiceServer::new(MyNtsService::default()))
                .serve(addr)
                .await
                .unwrap_or_else(|err| {
                    // Handle the error if creating the runtime fails
                    panic!("Error creating gRPC server: {}", err);
                });
        });
    }});

    let handle = thread::spawn(|| {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        let metrics = metrics::get_metrics();
        for line in stdin.lock().lines() {
            match line {
                Ok(input) => {
                    let parts: Vec<&str> = input.trim().split_whitespace().collect();
                    if parts.len() != 2 {
                        eprintln!("Please enter exactly two numbers separated by a space.");
                        continue;
                    }

                    let a = parts[0].parse::<i32>();
                    let b = parts[1].parse::<i32>();

                    match (a, b) {
                        (Ok(x), Ok(y)) => {
                            metrics.fec_row.set(x as i64);
                            metrics.fec_col.set(y as i64);
                        },
                        _ => eprintln!("Invalid input. Make sure both entries are integers."),
                    }
                }
                Err(e) => eprintln!("Error reading line: {}", e),
            }
        }
    });

    // Start the periodic task
    rt.spawn(clean_client_session(Arc::clone(&client_sessions)));
    rt.spawn(update_packet_loss_metrics(Arc::clone(&client_sessions)));
    
    // Wait for the thread to finish (e.g., when EOF is reached)
    if let Err(e) = handle.join() {
        panic!("handle panicked: {:?}", e);
    }
    // Join the threads and propagate any panics
    if let Err(e) = client_sessions_handler.join() {
        panic!("client_sessions_handler panicked: {:?}", e);
    }
    if let Err(e) = source_handler.join() {
        panic!("source_handler panicked: {:?}", e);
    }
    if let Err(e) = pipeline_handler.join() {
        panic!("pipeline_handler panicked: {:?}", e);
    }
    if let Err(e) = grpc_server_handler.join() {
        panic!("grpc_server_handler panicked: {:?}", e);
    }
    
    Ok(())
}

/// Start the Prometheus exporter
/// This function binds the exporter to the address specified in the configuration file
fn start_prometheus_exporter() {

    // Parse address used to bind exporter to.
    let prometheus_port = &CONFIG.get().unwrap().prometheus.port;
    let socket_addr = format!("0.0.0.0:{}", prometheus_port).parse().unwrap();

    // Start exporter and update metrics every five seconds.
    prometheus_exporter::start(socket_addr).expect("can not start exporter");

    // Initialize the metrics
    init_metrics();
}

/// Periodically clean up client sessions using `tokio::time::interval`
async fn clean_client_session(client_sessions: Arc<ClientSessions>) {
    let config = CONFIG.get().unwrap();
    let mut interval = tokio::time::interval(Duration::from_secs(config.settings.timeout_clients));

    loop {
        interval.tick().await;
        client_sessions.remove_timed_out_clients();
        info!("Cleaned up timed out clients");
    }
}

/// Periodically update packet loss metrics using `tokio::time::interval`
async fn update_packet_loss_metrics(client_sessions: Arc<ClientSessions>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let metrics = metrics::get_metrics();

    loop {
        interval.tick().await;

        // Calculate packet loss and update the metric
        let packet_loss = client_sessions.calculate_packet_loss();
        
        // If packet_loss is -1.0, it means that the client_sessions is empty
        if packet_loss != -1.0 {
            metrics.packet_loss.set(packet_loss.into());
        }
    }
}

/// ------------------------------------------ Matrix History -------------------------------------------------------------
/// A struct to hold the matrix data and its index
#[derive(Clone,Debug)]
pub struct IndexedMatrix {
    pub index: usize,
    pub data: Grid<TransmissionPacket>,
}

#[derive(Debug)]
pub struct MatrixHistory {
    matrices: Vec<RwLock<Option<IndexedMatrix>>>,
    capacity: usize,
    latest_index: RwLock<Option<usize>>,
}
impl MatrixHistory {
    pub fn new(capacity: usize) -> Self {
        MatrixHistory {
            matrices: (0..capacity)
                .map(|_| RwLock::new(None))
                .collect(),
            capacity,
            latest_index: RwLock::new(None),
        }
    }

    pub fn is_valid_index(&self, index: usize) -> bool {
        if let Some(latest) = *self.latest_index.read().unwrap() {
            index >= latest.saturating_sub(self.capacity - 1) && index <= latest
        } else {
            false
        }
    }

    pub fn add_matrix(&self, index: usize, grid: Grid<TransmissionPacket>) {
        let slot_index = index % self.capacity;
        let slot = &self.matrices[slot_index];
        let mut slot_guard = slot.write().unwrap();
        *slot_guard = Some(IndexedMatrix { index, data: grid });
    
        let mut latest_guard = self.latest_index.write().unwrap();
        *latest_guard = Some(index);
    }

    pub fn get_matrix(&self, index: usize) -> Option<Grid<TransmissionPacket>> {
        let slot_index = index % self.capacity;
        let slot = &self.matrices[slot_index];
        let guard = slot.read().unwrap();

        if let Some(ref matrix) = *guard {
            if matrix.index == index {
                return Some(matrix.data.clone());
            }
        }
        None
    }
}

