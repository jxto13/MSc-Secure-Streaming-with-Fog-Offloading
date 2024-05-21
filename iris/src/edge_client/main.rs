mod decoder;
mod receiver;
mod utils;

use std::sync::mpsc;

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

use log::{debug, error, log_enabled, info, Level};

pub const GRPC_SERVICE_PORT: u16 = 50052;

// Global variables
pub mod proto {
    tonic::include_proto!("streaming_server");
    tonic::include_proto!("edge_client");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("edge_client_descriptor");
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger conditionally
    #[cfg(not(target_arch = "aarch64-linux-android"))]
    {
        env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()

        .init();

        debug!("this is a debug {}", "message");
        error!("this is printed by default");

        if log_enabled!(Level::Info) {
            let x = 3 * 4; // expensive computation
            info!("the answer was: {}", x);
        }
    }
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

    iris::edge_client::test();
    let (listener_sender, matrix_decoder_receiver) = mpsc::channel();

    // Creates new thread for multicast receiver
    let listener_handler = std::thread::spawn(|| {
        receiver::multicast_receiver::start(
            "224.0.0.123".to_string(),
            "12345".to_string(),
            listener_sender,
        );
    });

    let matrix_decoder_handler = std::thread::spawn(|| {
        decoder::matrix::start(matrix_decoder_receiver);
    });

    // Join all the threads to ensure they complete before exiting
    listener_handler.join().expect("listener_handler panicked");
    matrix_decoder_handler
        .join()
        .expect("matrix_decoder_handler panicked");

    Ok(())
}
