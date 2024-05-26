use crossbeam_channel::{bounded, unbounded, Receiver};
use std::process::Command;

use std::thread;
use std::time::Duration;

use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use prost::Message;

use proto::certificate_service_client::CertificateServiceClient;
use proto::Data;
use proto::{CertificateRequest, SignedCertificateRequest};

use proto::FILE_DESCRIPTOR_SET;
use tokio::runtime::Runtime;
use tokio::task;

use tonic::transport::server::Server;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

use log::{debug, error, info, log_enabled, Level};

use super::decoder;
use super::receiver;

pub const GRPC_SERVICE_PORT: u16 = 50052;

// Global variables
pub mod proto {
    tonic::include_proto!("streaming_server");
    tonic::include_proto!("edge_client");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("edge_client_descriptor");
}

pub fn start() -> Result<Receiver<Vec<u8>>, Box<dyn std::error::Error>> {
    // // Connect to the server
    // let mut client = CertificateServiceClient::connect("http://127.0.0.1:50051").await?;

    // // Generate an RSA key pair
    // let rsa = Rsa::generate(2048)?;
    // // Create a PKey object from the RSA key pair
    // let key_pair = PKey::from_rsa(rsa)?;

    // // Make a X509 request with the given private key
    // let request = utils::mk_request(&key_pair).unwrap();

    // // Serialize the request CSR
    // let csr_vec = request.to_pem().unwrap();

    // // Make a CertificateRequest with the CSR
    // let csr_request = CertificateRequest {
    //     csr: csr_vec,
    // };

    // // Send the CSR to the server to get it signed
    // let response = client.sign_certificate(csr_request).await?;
    // let signed_cert = response.into_inner().signed_cert;

    // // Assuming cert_bytes is the bytes of the received certificate
    // let cert_request = SignedCertificateRequest { cert: signed_cert };

    // // Send the signed certificate to the server for verification
    // let validation_response = client.verify_certificate(cert_request).await?;
    // let auth = validation_response.into_inner().auth;

    // if auth {
    //     println!("Certificate verified successfully!");
    // } else {
    //     println!("Certificate verification failed!");
    //     return Err("An error occurred".into());
    // }

    let (listener_sender, matrix_decoder_receiver) = unbounded();
    let (decoded_data_sender, decoded_data_receiver) = unbounded();


     // Creates new thread for multicast receiver
    let listener_handler = std::thread::spawn(move || {
        info!("Starting multicast receiver on address 224.0.0.123:12345.");
        match receiver::multicast_receiver::start(
            "239.1.1.10".to_string(),
            "12345".to_string(),
            listener_sender,
        ) {
            Ok(_) => {
                info!("Multicast receiver exited successfully.");
            }
            Err(e) => {
                error!("Multicast receiver failed with error: {}", e);
            }
        }
    });

    // Clone channel to receiver thread

    let matrix_decoder_handler = thread::spawn(move || {
        
        if let Err(e) = decoder::matrix::start(matrix_decoder_receiver, decoded_data_sender) {
            error!("Matrix decoder failed to start: {}", e);
        }
            
    });


    // Spawn a separate thread for joining and returning the sender
    thread::spawn(move || {
        // Join all the threads to ensure they complete before returning
        listener_handler.join().expect("listener_handler panicked");
        matrix_decoder_handler
            .join()
            .expect("matrix_decoder_handler panicked");
    });

    Ok(decoded_data_receiver)
}
