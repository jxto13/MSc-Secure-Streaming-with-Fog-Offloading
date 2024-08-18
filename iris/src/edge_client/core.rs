use crossbeam_channel::{bounded, unbounded, Receiver};
use once_cell::sync::OnceCell;
use openssl::bn::BigNumContext;
use openssl::ec::{EcGroup, EcKey, EcPoint};
use openssl::nid::Nid;
use openssl::x509::X509;
use prost::Message;
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;
use std::{fs, thread};
use tokio::sync::mpsc;

use openssl::pkey::{PKey, Private, Public};
use openssl::rsa::Rsa;

use proto::certificate_service_client::CertificateServiceClient;
use proto::{CertificateRequest, SignedCertificateRequest};

use tokio::runtime::Runtime;
use tokio::time::sleep;

use tonic::transport::{Certificate, Channel, ClientTlsConfig};

use log::{error, info};

use crate::edge_client::proto::SessionKey;

use super::proto::key_service_client::KeyServiceClient;
use super::proto::{RequestKey,KeyResponse};
use super::receiver;
use super::session;
use super::{decoder, proto, utils};

const CA_PUBLIC_CERTIFICATE: &[u8] = include_bytes!("../../certs/ca_certificate.pem");

pub static CHANNEL: OnceCell<Channel> = OnceCell::new();
pub static ECDSA_KEY: OnceCell<Arc<Mutex<EcKey<Public>>>> = OnceCell::new();

pub fn start() -> Result<Receiver<Vec<u8>>, Box<dyn std::error::Error>> {
    // Set up a Tokio runtime
    let rt = Arc::new(Runtime::new().unwrap());

    let current_session_key: Arc<Mutex<SessionKey>> = Arc::new(Mutex::new(SessionKey { key: vec![0, 0], iv: vec![0, 0] }));
    let certificate: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![0,0]));
    
    {
        let runtime = Arc::clone(&rt);

        // Run async code using the Tokio runtime
        runtime.block_on(async {

            let current_session_key = Arc::clone(&current_session_key);
            let certificate = Arc::clone(&certificate);

            loop {
                match create_channel().await {
                    Ok(channel) => {
                        let mut connection = CertificateServiceClient::new(channel.clone());

                        info!("Connected to GRPC Server");

                        // Generate an RSA key pair
                        let rsa = Rsa::generate(2048).unwrap();

                        // Create a PKey object from the RSA key pair
                        let key_pair = PKey::from_rsa(rsa).unwrap();

                        // Make a X509 request with the given private key
                        let request = utils::mk_request(&key_pair).unwrap();

                        // Serialize the request CSR
                        let csr_vec = request.to_pem().unwrap();

                        // Make a CertificateRequest with the CSR
                        let csr_request = CertificateRequest { csr: csr_vec };

                        // Send the CSR to the server to get it signed
                        let response = connection
                            .sign_certificate(csr_request)
                            .await
                            .unwrap();
                        let signed_cert = response.into_inner().signed_cert;

                        // Assuming cert_bytes is the bytes of the received certificate
                        let cert_request = SignedCertificateRequest { cert: signed_cert };

                        // Send the signed certificate to the server for verification
                        let validation_response = connection
                            .verify_certificate(cert_request)
                            .await
                            .unwrap();

                        let auth = validation_response.into_inner().auth;

                        if auth {
                            info!("Certificate verified successfully!");
                        } else {
                            error!("Certificate verification failed!");
                        }

                        // Save the client for use after the loop
                        CHANNEL.set(channel).unwrap();


                        let mut connection = KeyServiceClient::new(CHANNEL.get().unwrap().clone());

                        let key_response = connection.request_current_key(RequestKey {}).await.unwrap();

                        let reponse = key_response.into_inner();

                        info!("key: {:?}, certificate: {:?}", reponse.current_key, reponse.certificate);

                        if let Some(current_key) = reponse.current_key {
                            let mut current_session_key = current_session_key.lock().unwrap();
                            current_session_key.key = current_key.key;
                            current_session_key.iv = current_key.iv;
                        }

                        let mut certificate_mutex = certificate.lock().unwrap();
                        *certificate_mutex = reponse.certificate;

                        break; // Exit the loop once connected
                    }
                    Err(e) => {
                        error!("Failed to connect: {}. Retrying in 3 seconds...", e);
                        sleep(Duration::from_secs(3)).await;
                    }
                }
            }
        });
    }

    // If we arrive here we have certificates signed from PKI

    // Create channels for thread comunication
    let (listener_sender, matrix_decoder_receiver) = unbounded();
    let (decoded_data_sender, decoded_data_receiver) = unbounded();
    let (client_report_sender, client_report_receiver) = unbounded();

    // Create a Barrier for thread synchronization
    let barrier = Arc::new(Barrier::new(2));

    // Clone the Barrier for the listener thread
    let listener_barrier = Arc::clone(&barrier);

    // Creates new thread for multicast receiver
    let listener_handler = std::thread::spawn(move || {
        info!("Starting multicast receiver on address 239.255.0.1:12345.");
        match receiver::multicast_receiver::start(
            "239.255.0.1".to_string(),
            "12345".to_string(),
            listener_sender,
            listener_barrier,
        ) {
            Ok(_) => {
                info!("Multicast receiver exited successfully.");
            }
            Err(e) => {
                error!("Multicast receiver failed with error: {}", e);
            }
        }
    });

    // Cloning the value inside the current_session_key Mutex
    let cloned_session_key: SessionKey = {
        let session_key = current_session_key.lock().unwrap();
        session_key.clone()
    };

    // Cloning the value inside the certificate Mutex
    let cloned_certificate: Vec<u8> = {
        let cert = certificate.lock().unwrap();
        cert.clone()
    };

    // Clone the Barrier for the decoder thread
    let decoder_barrier = Arc::clone(&barrier);

    // dont need handler because when the listener goes out of scope, i want the decoder to also go out of scoupe
    thread::spawn(move || {
        // Wait for the listener multicast with a barrier before starting the decoder
        decoder_barrier.wait();
        if let Err(e) = decoder::start(
            matrix_decoder_receiver,
            decoded_data_sender,
            client_report_sender,
            cloned_session_key,
            cloned_certificate,
            2
        ) {
            error!("Matrix decoder failed to start: {}", e);
        }
    });

    let runtime = Arc::clone(&rt);
    // Spawn Client Report Thread
    thread::spawn(move || {
        // Wait for the listener multicast with a barrier before starting the decoder
        let mut client_report = session::ClientSession::new(runtime);
        client_report.start_report_client(client_report_receiver);
    });

    // Spawn a separate thread for joining and returning the sender
    thread::spawn(move || {
        // Join all the threads to ensure they complete before returning
        listener_handler.join().expect("listener_handler panicked");
    });

    Ok(decoded_data_receiver)
}

async fn create_channel() -> Result<Channel, Box<dyn std::error::Error>> {
    // Configure the TLS settings for the channel
    let tls_config = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(&CA_PUBLIC_CERTIFICATE))
        .domain_name("www.example.com");

    // Create the channel with the TLS configuration
    let channel = Channel::from_static("https://razer15:50051")
        .tls_config(tls_config)?
        .connect()
        .await?;

    // If we reach here, connection was successful
    Ok(channel)
}
