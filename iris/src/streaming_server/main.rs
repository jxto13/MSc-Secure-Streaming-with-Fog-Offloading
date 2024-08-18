mod crypto;
mod network_scheduler;
mod source_handler;
mod membership_manager;
mod certificates;

use std::sync::Mutex;
use std::{sync::Arc, thread};
use std::path::PathBuf;

// Standard
use crossbeam_channel::bounded;
use crypto::key_manager::KeyManager;
use iris::streaming_server::proto::report_service_server::ReportServiceServer;
use iris::streaming_server::proto::{self, FILE_DESCRIPTOR_SET};
use log::{error, info};
use membership_manager::reporting_sink::MyReportService;
use once_cell::sync::OnceCell;
use openssl::x509::X509;
use source_handler::data_handler::{DataHandler,DataSource};

use tonic::transport::{Identity, Server, ServerTlsConfig};
use tokio::runtime::Runtime;

use iris::streaming_server::{read_config, Config};

// Define the global static instance of Config using OnceCell
static CONFIG: OnceCell<Config> = OnceCell::new();
static MANAGER: OnceCell<Arc<Mutex<KeyManager>>> = OnceCell::new();

// Custom
use network_scheduler::pipeline::Pipeline;
use clap::{arg, Command};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define the command structure
    let cmd = Command::new("streaming_server")
        .version("0.0.1")
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

    // ----------------------------------- Programs Setup -------------------------------------------------------------

    // Load or create certificates
    let certificates = certificates::load_or_create_certificates()?;
    let key_manager = Arc::new(Mutex::new(KeyManager::new(certificates.ec_key)));

    // Start key_service before the pipeline to generate the ECDSA Keys for encoder 
    let key_service = crypto::key_service::get_service(Arc::clone(&key_manager), X509::from_pem(certificates.ec_cert.as_bytes()).unwrap());

    MANAGER.set(key_manager).unwrap();

    // ----------------------------------- Programs Starts Here ------------------------------------------------------

    // Create channels
    let (source_handler_sender, source_handler_receiver) = bounded(1);
    
    // Choose the data source
    let source = DataSource::File(CONFIG.get().unwrap().multicast.file_path.clone());

    // Create a DataHandler instance based on datasource
    let data_handler = DataHandler::new(source, source_handler_sender);        
    
    let source_handler = thread::spawn(move || {
        data_handler.start();
    });
    
    // Create the pipeline
    let pipeline= Arc::new(Pipeline::new());

    let pipeline_handler = {
        let pipeline = Arc::clone(&pipeline);
        thread::spawn(move || {
            pipeline.start(source_handler_receiver);
        })
    };

    // Create a std thread and setup tokio for gRPC Server with tokio runtime
    let grpc_server_handler = thread::spawn(move || {
        // Inside the thread, set up a Tokio runtime
        let rt = Runtime::new().unwrap();

        // Run async code using the Tokio runtime
        rt.block_on(async {
            
            // listen to all ipv6 addresses
            let addr = format!("0.0.0.0:{}", CONFIG.get().unwrap().grpc.port).parse().unwrap();
            
            // Start grpc server this will block execution because of await
            Server::builder()
                .tls_config(ServerTlsConfig::new()
                    .identity(Identity::from_pem(certificates.server_cert, certificates.server_key))).unwrap()
                .concurrency_limit_per_connection(256)
                // Service reflection to enable clients to see services
                .add_service(tonic_reflection::server::Builder::configure().register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET).build().unwrap())
                .add_service(crypto::ca_service::get_service(certificates.ca_cert, certificates.ca_private_key)) // Responsible for signing the client certificates
                .add_service(key_service)
                .add_service(ReportServiceServer::new(MyReportService::new(pipeline.get_arc_client_sessions())))
                .serve(addr)
                .await
                .unwrap_or_else(|err| {
                    // Handle the error if creating the runtime fails
                    error!("Error creating gRPC server: {}", err);
                });
        });
    });

    source_handler.join().expect("source_handler panicked");
    pipeline_handler.join().expect("pipeline_handler panicked");
    grpc_server_handler.join().expect("grpc_server_handler panicked");
    Ok(())
}
