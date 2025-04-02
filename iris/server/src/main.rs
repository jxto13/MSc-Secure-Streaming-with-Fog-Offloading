mod crypto;
mod network_scheduler;
mod source_handler;
mod membership_manager;
mod metrics;
mod nts_service;

use std::sync::RwLock;
use std::{sync::Arc, thread};
use std::path::PathBuf;

use circular_buffer::CircularBuffer;
// Standard
use crossbeam_channel::unbounded;
use crypto::key_manager::KeyManager;
use grid::Grid;
use log::error;
use membership_manager::client_sessions;
use membership_manager::reporting_sink::MyReportService;
use crypto::certificates;
use once_cell::sync::OnceCell;
use openssl::x509::X509;
use proto::nts_service_server::NtsServiceServer;
use source_handler::data_handler::{DataHandler,DataSource};
use std::time::Duration;

use tonic::transport::{Identity, Server, ServerTlsConfig};
use tokio::runtime::Runtime;

use proto::TransmissionPacket;
use proto::report_service_server::ReportServiceServer;

use iris_server::{read_config,Config};

use client_sessions::ClientSessions;
use crate::nts_service::MyNtsService;
use crate::metrics::init_metrics;


// Import protobuff
pub mod proto {
    tonic::include_proto!("streaming_server");
}

pub const MAX_BUFFER_SIZE: usize = 10000;

// Define the global static instance of Config using OnceCell
static CONFIG: OnceCell<Config> = OnceCell::new();
static MANAGER: OnceCell<Arc<RwLock<KeyManager>>> = OnceCell::new();
static MATRIX_BUFFER: OnceCell<Arc<RwLock<CircularBuffer::<MAX_BUFFER_SIZE, Option<Grid<TransmissionPacket>>>>>> = OnceCell::new();
static FEC_PARAMS: OnceCell<Arc<RwLock<(usize, usize)>>> = OnceCell::new();

// Custom
use network_scheduler::pipeline::Pipeline;
use clap::{arg, Command};


fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Read configuration file
    let config = read_config(config_filename.to_str().unwrap())?;

    // Set the global CONFIG instance
    CONFIG.set(config).expect("Failed to set CONFIG");

    MATRIX_BUFFER.set(Arc::new(RwLock::new(CircularBuffer::<MAX_BUFFER_SIZE, Option<Grid<TransmissionPacket>>>::new()))).expect("Failed to set MATRIX_BUFFER");
    FEC_PARAMS.set(Arc::new(RwLock::new((CONFIG.get().unwrap().multicast.row_fec, CONFIG.get().unwrap().multicast.col_fec)))).expect("Failed to set FEC_PARAMS");
    
    start_prometheus_exporter();

    // Set up a Tokio runtime
    let rt = Arc::new(Runtime::new().unwrap());
    
    // ----------------------------------- Programs Setup -------------------------------------------------------------

    // Load or create certificates
    let certificates = certificates::load_or_create_certificates()?;
    let key_manager = Arc::new(RwLock::new(KeyManager::new(certificates.ec_key)));

    // Start key_service before the pipeline to generate the ECDSA Keys for encoder 
    let key_service = crypto::key_service::get_service(Arc::clone(&key_manager), X509::from_pem(certificates.ec_cert.as_bytes()).unwrap());

    MANAGER.set(key_manager).unwrap();

    // ----------------------------------- Programs Starts Here ------------------------------------------------------

    // Create channels
    let (source_handler_sender, source_handler_receiver) = unbounded();
    
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
    let client_sessions = Arc::new(RwLock::new(ClientSessions::new()));

    // Start client_sessions thread
    let client_sessions_handler = thread::spawn({
        let client_sessions = Arc::clone(&client_sessions);
        move || {
            let timeout = Duration::from_secs(CONFIG.get().unwrap().settings.timeout_clients);
            // Check every N second for timeout clients
            loop {
                {
                    // Lock the client_sessions for writing
                    let mut client_sessions = client_sessions.write().unwrap();
                    // Removing timed-out clients
                    client_sessions.remove_timed_out_clients();
                }
                // Sleep outside the lock to avoid holding it during the sleep
                thread::sleep(timeout);
            }
        }
    });

    // Create the pipeline
    let pipeline= Arc::new(Pipeline::new(Arc::clone(&client_sessions),CONFIG.get().unwrap().retransmission.wifi_multicast, CONFIG.get().unwrap().retransmission.wifi_unicast));

    let pipeline_handler = {
        let pipeline = Arc::clone(&pipeline);
        thread::spawn(move || {
            pipeline.start(source_handler_receiver);
        })
    };

    // Create a std thread and setup tokio for gRPC Server with tokio runtime
    let grpc_server_handler = thread::spawn({
        let rt = Arc::clone(&rt);
        move || {
        // Run async code using the Tokio runtime
        rt.block_on(async {

            // listen to all ipv4 addresses
            let addr = format!("0.0.0.0:{}", CONFIG.get().unwrap().grpc.port).parse().unwrap();
            
            // Start grpc server this will block execution because of await
            Server::builder()
                .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(certificates.server_cert, certificates.server_key))).unwrap()
                .add_service(crypto::ca_service::get_service(certificates.ca_cert, certificates.ca_private_key)) // Responsible for signing the client certificates
                .add_service(key_service)
                .add_service(ReportServiceServer::new(MyReportService::new(Arc::clone(&client_sessions))))
                .add_service(NtsServiceServer::new(MyNtsService::default()))
                .serve(addr)
                .await
                .unwrap_or_else(|err| {
                    // Handle the error if creating the runtime fails
                    error!("Error creating gRPC server: {}", err);
                });
        });
    }});

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

fn start_prometheus_exporter() {

    // Parse address used to bind exporter to.
    let prometheus_port = &CONFIG.get().unwrap().prometheus.port;
    let socket_addr = format!("0.0.0.0:{}", prometheus_port).parse().unwrap();

    // Start exporter and update metrics every five seconds.
    prometheus_exporter::start(socket_addr).expect("can not start exporter");

    // Initialize the metrics
    init_metrics();
}