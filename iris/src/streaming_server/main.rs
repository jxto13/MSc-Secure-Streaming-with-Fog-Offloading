mod crypto;
mod network_scheduler;
mod source_handler;

use std::io;
use std::{sync::Arc, thread};
use std::path::PathBuf;

// Standard
use crossbeam_channel::unbounded;
use iris::streaming_server::proto::{self, FILE_DESCRIPTOR_SET};
use log::{error, info};
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::X509;
use source_handler::data_handler::{DataHandler,DataSource};

use tonic::transport::{Identity, Server, ServerTlsConfig};
use tokio::runtime::Runtime;

use iris::streaming_server::read_config;

// Custom
use network_scheduler::pipeline::Pipeline;
use clap::{arg, Command};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define the command structure
    let cmd = Command::new("streaming_server")
        .version("1.0")
        .author("Your Name <your.email@example.com>")
        .about("A streaming server with gRPC and multicast capabilities")
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

    // Certificates file path
    let cert_file_path = "./certs/ca_certificate.pem";
    let key_file_path = "./certs/ca_private_key.pem";

    let (ca_cert, ca_private_key): (X509, PKey<Private>);
    
    // Generate or load CA Certificates 
    if crypto::utils::file_exists(cert_file_path) && crypto::utils::file_exists(key_file_path) {
        // Load the certificate and private key from the files
        match (crypto::utils::load_certificate_from_file(cert_file_path), crypto::utils::load_private_key_from_file(key_file_path)) {
            (Ok(cert), Ok(pkey)) => {
                info!("Loaded certificate and private key from files:");
                info!("Certificate: {:?}", cert);
                info!("Private Key: {:?}", pkey);
                ca_cert = cert;
                ca_private_key = pkey;
            }
            (Err(e), _) => {
                error!("Failed to load certificate: {}", e);
                return Err(Box::new(e));
            }
            (_, Err(e)) => {
                error!("Failed to load private key: {}", e);
                return Err(Box::new(e));
            }
        }
    } else {
        let (cert, key_pair) = crypto::utils::mk_ca_cert()?;
        crypto::utils::save_certificate_to_file(&cert, cert_file_path)?;
        crypto::utils::save_private_key_to_file(&key_pair, key_file_path)?;
        info!("Saved new certificate and private key to files.");
        ca_cert = cert;
        ca_private_key = key_pair;
    }

    // Certificates file path
    let server_cert_file_path = "./certs/server_certificate.pem";
    let server_private_file_path = "./certs/server_private_key.pem";

    let rsa = Rsa::generate(2048).unwrap();
    let pkey = PKey::from_rsa(rsa).unwrap();

    let request = crypto::utils::mk_request(&pkey).unwrap();
    
    // Sign the CSR with the CA certificate
    let signed_cert = crypto::utils::sign_request_with_ca(&request, &ca_cert, &ca_private_key)?;

    crypto::utils::save_certificate_to_file(&signed_cert, server_cert_file_path)?;
    crypto::utils::save_private_key_to_file(&pkey, server_private_file_path)?;

    let cert = std::fs::read_to_string("./certs/server_certificate.pem")?;
    let key = std::fs::read_to_string("./certs/server_private_key.pem")?;

    info!("{:?} \n\n\n {}",signed_cert,  String::from_utf8(signed_cert.to_pem().unwrap()).unwrap());

    let multicast_ip = config.multicast_ip;

    let port = config.port;
    let (source_handler_sender, source_handler_receiver) = unbounded();
    
    // Choose the data source
    let source = DataSource::File(config.url_stream);

    // Create a DataHandler instance
    let data_handler = DataHandler::new(source, source_handler_sender);        
    
    let source_handler = thread::spawn(move || {
        data_handler.start();
    });
    
    // Create the pipeline
    let pipeline = Arc::new(Pipeline::new());
    
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
            let addr = format!("0.0.0.0:{}", config.grpc_service_port).parse().unwrap();
            
            let private_key = pkey.private_key_to_pem_pkcs8().unwrap();
            // Start grpc server this will block execution because of await
            Server::builder()
                .tls_config(ServerTlsConfig::new()
                .identity(Identity::from_pem(cert, key))).unwrap()
                .concurrency_limit_per_connection(256)
                // Service reflection to enable clients to see services
                .add_service(tonic_reflection::server::Builder::configure().register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET).build().unwrap())
                .add_service(crypto::service::get_service(ca_cert, ca_private_key)) // Responsible for signing the client certificates
                .add_service(pipeline.get_client_sessions_service())
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
