use super::utils::SessionConfig;
use super::{receiver, session, decoder, utils};

use crossbeam_channel::{unbounded, Receiver};

use once_cell::sync::OnceCell;

use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::x509::X509;
use openssl::pkey::PKey;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use crate::edge_client::proto::{RequestKey,CertificateRequest};
use crate::edge_client::proto::key_service_client::KeyServiceClient;
use crate::edge_client::proto::certificate_service_client::CertificateServiceClient;

use tokio::runtime::Runtime;
use tokio::time::sleep;

use tonic::transport::{Certificate, Channel, ClientTlsConfig};

use log::{error, info};

const CA_PUBLIC_CERTIFICATE: &[u8] = include_bytes!("../../certs/ca_certificate.pem");

pub static CHANNEL: OnceCell<Channel> = OnceCell::new();
pub static INTERFACES: OnceCell<Arc<Mutex<Vec<String>>>> = OnceCell::new();

pub fn start() -> Result<Receiver<Vec<u8>>, Box<dyn std::error::Error>> {
    // Initialize INTERFACES if it's not already initialized
    INTERFACES.get_or_init(|| Arc::new(Mutex::new(Vec::new())));
    
    // Set up a Tokio runtime
    let rt = Arc::new(Runtime::new().unwrap());

    let session_config: Arc<Mutex<SessionConfig>> = Arc::new(Mutex::new(SessionConfig::default()));
    
    // EC Certificates
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ec_key = EcKey::generate(&group).unwrap();

    {
        let runtime = Arc::clone(&rt);

        // Run async code using the Tokio runtime
        runtime.block_on(async {

            let arc_session_config = Arc::clone(&session_config);

            loop {
                match create_channel().await {
                    Ok(channel) => {
                        let mut connection = CertificateServiceClient::new(channel.clone());

                        // Extract key-pair to struct
                        let pkey = PKey::from_ec_key(ec_key.clone()).unwrap();
                        
                        // Make a X509 request with the given private key
                        let request = utils::mk_request(&pkey).unwrap();

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

                        // Convert from Vec<u8> to X509
                        let cert = X509::from_pem(&signed_cert).unwrap();
                        
                        let result = X509::from_pem(CA_PUBLIC_CERTIFICATE).unwrap().issued(&cert);

                        info!("Certificate issued from CA: {}", result);

                        let mut connection = KeyServiceClient::new(channel.clone());

                        let key_response = connection.request_current_key(RequestKey {}).await.unwrap();

                        let reponse = key_response.into_inner();

                        let mut mutex_session_config = arc_session_config.lock().unwrap();
                        
                        *mutex_session_config = SessionConfig {
                            current_key: reponse.current_key.unwrap(),
                            ec_server_certificate: reponse.certificate,
                            epoch: reponse.epoch,
                            client_ec_certificate: signed_cert,
                            client_ec_key: Some(ec_key)
                        };

                        // Save the client for use after the loop
                        CHANNEL.set(channel).unwrap();

                        break; // Exit the loop once connected
                    }
                    Err(e) => {
                        error!("Failed to connect: {}. Retrying in 5 seconds...", e);
                        sleep(Duration::from_secs(5)).await;
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

    // Creates new thread for multicast receiver
    let listener_handler = std::thread::spawn(move || {
        info!("Starting multicast receiver on address 239.255.0.1:12345.");
        match receiver::multicast_receiver::start(
            "239.255.0.1".to_string(),
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

    // Extract the session config from the Mutex
    let cloned_session_config = {
        let mutex_session_config = session_config.lock().unwrap();
        (*mutex_session_config).clone() // Clone the session config to pass it
    };

    // dont need handler because when the listener goes out of scope, i want the decoder to also go out of scoupe
    let decoder_handler = thread::spawn(move || {

        if let Err(e) = decoder::start(
            matrix_decoder_receiver,
            decoded_data_sender,
            client_report_sender,
            cloned_session_config,
            10
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
        decoder_handler.join().expect("decoder_handler panicked");
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

    info!("Connected to GRPC Server");

    // If we reach here, connection was successful
    Ok(channel)
}