mod crypto;
mod network_scheduler;
mod source_handler;

use std::thread;
use std::path::PathBuf;

// Standard
use crossbeam_channel::unbounded;
use iris::streaming_server::proto::{self, FILE_DESCRIPTOR_SET};
use log::info;
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

    // Generate CA Certificates and stores in memory
    let (ca_cert, ca_key_pair) = crypto::utils::mk_ca_cert()?;

    let multicast_ip = config.multicast_ip.clone();

    let mut session_handlers = vec![];

    for session in config.sessions {
        let url_stream = session.url_stream.clone();
        let port = session.port;
        let (source_handler_sender, source_handler_receiver) = unbounded();

        // Choose the data source
        let source = DataSource::File(url_stream);

        // Create a DataHandler instance
        let data_handler = DataHandler::new(source, source_handler_sender);        
        
        let source_handler = thread::spawn(move || {
            data_handler.start();
        });

        // Create the pipeline
        let pipeline = Pipeline::new();

        let pipeline_handler = thread::spawn(move || {
            pipeline.start(source_handler_receiver);
        });

        session_handlers.push(source_handler);
        session_handlers.push(pipeline_handler);
    }

    // Create a std thread and setup tokio for gRPC Server with tokio runtime
    let grpc_server_handler = thread::spawn(move || {
        // Inside the thread, set up a Tokio runtime
        let rt = Runtime::new().unwrap();

        // Run async code using the Tokio runtime
        rt.block_on(async {
            // Service reflection to enable clients to see services
            let service = tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
                .build()
                .unwrap();

            // listen to all ipv6 addresses
            let addr = format!("0.0.0.0:{}", config.grpc_service_port).parse().unwrap();

            // Start grpc server this will block execution because of await
            Server::builder()
                .add_service(service)
                .add_service(crypto::service::get_service(ca_cert, ca_key_pair)) // Responsible for signing the client certificates
                .serve(addr)
                .await
                .unwrap_or_else(|err| {
                    // Handle the error if creating the runtime fails
                    eprintln!("Error creating gRPC server: {}", err);
                });
        });
    });

    // Join all the session handlers to ensure they complete before exiting
    for handler in session_handlers {
        handler.join().expect("source_handler or pipeline_handler panicked");
    }

    grpc_server_handler.join().expect("grpc_server_handler panicked");

    Ok(())
}
