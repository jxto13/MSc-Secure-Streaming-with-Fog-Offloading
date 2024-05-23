#![allow(unused)]
mod crypto;
mod network_scheduler;
mod source_handler;

use std::process::Stdio;
use std::io::BufRead;
use std::io::BufReader;
use std::io::ErrorKind;

// Standard
use std::{process::Command, sync::mpsc};
use std::thread;

use proto::FILE_DESCRIPTOR_SET;
use tonic::transport::{Identity, Server, ServerTlsConfig};

use tokio::runtime::Runtime;

// Custom
use network_scheduler::pipeline::Pipeline;

// Global variables
pub const GRPC_SERVICE_PORT: u16 = 50051;

// Import protobuff
pub mod proto {
    tonic::include_proto!("streaming_server");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("streaming_server_descriptor");
}

// Main app, can be changed depending on platform
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate CA Certificates and stores in memory
    let (ca_cert, ca_key_pair) = crypto::utils::mk_ca_cert()?;

    let url_stream = "./public/demo.ts";

    // Channel communication between threads
    let (source_handler_sender, source_handler_receiver) = mpsc::channel();
    // -------------------------------------

    let source_handler = thread::spawn(|| {
        source_handler::file_source::start(url_stream, source_handler_sender);
    });

    let pipeline = Pipeline::new();

    let pipeline_handler = thread::spawn(move || {
        pipeline.start(source_handler_receiver);
    });

    // Create a std thread and setup tokio for gRPC Server with tokio runtime
    let grpc_server_handler = thread::spawn(|| {
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
            let addr = format!("0.0.0.0:{}", GRPC_SERVICE_PORT).parse().unwrap();

            // Start grpc server this will block execution because of await
            Server::builder()
                .add_service(service)
                .add_service(crypto::service::get_service(ca_cert, ca_key_pair)) // Responsable for signing the client certificates
                .serve(addr)
                .await
                .unwrap_or_else(|err| {
                    // Handle the error if creating the runtime fails
                    eprintln!("Error creating gRPC sercer: {}", err);
                });
        });
    });

    // Join all the threads to ensure they complete before exiting
    source_handler.join().expect("source_handler panicked");
    pipeline_handler.join().expect("pipeline_handler panicked");
    grpc_server_handler
        .join()
        .expect("grpc_server_handler panicked");

    Ok(())
}
