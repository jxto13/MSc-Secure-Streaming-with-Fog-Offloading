mod utils;
mod session;
mod decoder;
mod receiver;

use crossbeam_channel::{unbounded, Receiver};

use once_cell::sync::OnceCell;

use crate::utils::SessionConfig;

use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::x509::X509;
use openssl::pkey::PKey;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use proto::{RequestKey,CertificateRequest};
use proto::key_service_client::KeyServiceClient;
use proto::certificate_service_client::CertificateServiceClient;
use proto::nts_service_client::NtsServiceClient;
use proto::TimestampRequest;

use tokio;

use tokio::runtime::Runtime;
use tokio::time::sleep;

use tonic::transport::{Certificate, Channel, ClientTlsConfig};

use chrono::{DateTime, Utc};

use log::{error, info};

pub mod proto {
    tonic::include_proto!("streaming_server");
    tonic::include_proto!("edge_client");
}

const CA_PUBLIC_CERTIFICATE: &[u8] = include_bytes!("../../server/certs/ca_certificate.pem");

pub static CHANNEL: OnceCell<Channel> = OnceCell::new();
pub static TIME: OnceCell<Mutex<DateTime<Utc>>> = OnceCell::new();

pub fn start(ip: &String) -> Result<Receiver<Vec<u8>>, Box<dyn std::error::Error>> {
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
                match create_channel(ip).await {
                    Ok(channel) => {
                        // Create connection to retrieve client certificate from CA
                        {
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
    
                            let mut mutex_session_config = arc_session_config.lock().unwrap();

                            mutex_session_config.client_ec_certificate = signed_cert;
                            mutex_session_config.client_ec_key = Some(ec_key);
                        }

                        // Create connection to retrive current Session encryption key and certificate to verify packets is comming from the server
                        {
                            let mut connection = KeyServiceClient::new(channel.clone());
    
                            let key_response = connection.request_current_key(RequestKey {}).await.unwrap();
    
                            let reponse = key_response.into_inner();

                            let mut mutex_session_config = arc_session_config.lock().unwrap();

                            mutex_session_config.current_key = reponse.current_key.unwrap();
                            mutex_session_config.next_key = reponse.next_key.unwrap();
                            mutex_session_config.ec_server_certificate = reponse.certificate;
                            mutex_session_config.epoch = reponse.epoch;

                            info!("Current key: {:?}", mutex_session_config.current_key);
                        }

                        // Create connection to retrieve current timestamp to base our client report intervals
                        {
                            let mut connection = NtsServiceClient::new(channel.clone());
                            let stamp_response = connection.get_current_timestamp(TimestampRequest {}).await.unwrap();
                            let timestamp_str = stamp_response.into_inner().timestamp;
                            TIME.set(Mutex::new(DateTime::parse_from_rfc3339(&timestamp_str).unwrap().into())).unwrap();
                        }

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

    // If we arrive here we have certificates signed from PKI, Session Key and Current Timestamp

    // Create channels for thread comunication
    let (multicast_receiver_sender, multicast_receiver_receiver) = unbounded();
    let (unicast_receiver_sender, unicast_receiver_receiver) = unbounded();
    let (decoded_data_sender, decoded_data_receiver) = unbounded();
    let (client_report_sender, client_report_receiver) = unbounded();

    // Create the Client Session using the runtime that was init in the start
    let mut client_session = session::ClientSession::new(Arc::clone(&rt));

    // Creates new thread for multicast receiver
    let multicast_receiver_handler = std::thread::spawn(move || {
        info!("Starting multicast receiver on address 224.0.0.2:12345.");
        receiver::multicast_receiver::server(
            "224.0.0.2".to_string(),
            "12345".to_string(),
            multicast_receiver_sender,
        ).unwrap();
    });

    // Creates new thread for multicast receiver
    let unicast_receiver_handler = std::thread::spawn(move || {
        info!("Starting unicast receiver on address 224.0.0.2:12345.");
        // receiver::unicast_receiver::start(
        //     "224.0.0.2".to_string(),
        //     "12345".to_string(),
        //     multicast_receiver_sender,
        // ).unwrap();
    });

    // Dont need handler because when the listener goes out of scope the threads continue execution (infinite loops inside), i want the decoder to also go out of scope
    let decoder_handler = thread::spawn(move || {

        if let Err(e) = decoder::start(
            multicast_receiver_receiver,
            unicast_receiver_receiver,
            decoded_data_sender,
            client_report_sender,
            session_config.lock().unwrap().clone()
        ) {
            error!("Matrix decoder failed to start: {}", e);
        }
    });


    // Spawn Client Report Thread
    thread::spawn(move || {
        // Wait for the listener multicast with a barrier before starting the decoder
        client_session.start_report_client(client_report_receiver);
    });

    // Spawn a separate thread for joining and returning the sender
    thread::spawn(move || {
        // Join all the threads to ensure they complete before returning
        multicast_receiver_handler.join().expect("listener_handler panicked");
        decoder_handler.join().expect("decoder_handler panicked");
        unicast_receiver_handler.join().expect("unicast_handler panicked");
    });

    Ok(decoded_data_receiver)
}

async fn create_channel(ip: &String) -> Result<Channel, Box<dyn std::error::Error>> {
    // Configure the TLS settings for the channel
    let tls_config = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(&CA_PUBLIC_CERTIFICATE))
        .domain_name("www.example.com");

    // Create the channel with the TLS configuration using the provided IP address
    let channel_url = format!("https://{}:50051", ip);
    
    // Use Channel::builder instead of Channel::from_static
    let channel = Channel::builder(channel_url.parse()?) // Convert String to Uri
        .tls_config(tls_config)? // Apply the TLS configuration
        .connect()
        .await?;

    info!("Connected to GRPC Server at {}", ip);

    // If we reach here, connection was successful
    Ok(channel)
}
