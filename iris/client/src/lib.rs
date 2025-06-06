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

use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
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
    include!("../proto/generated/streaming_server.rs");
    include!("../proto/generated/edge_client.rs");
}

const CA_PUBLIC_CERTIFICATE: &[u8] = include_bytes!("../../server/certs/ca_certificate.pem");

pub static CHANNEL: OnceCell<Channel> = OnceCell::new();
pub static TIME: OnceCell<Mutex<DateTime<Utc>>> = OnceCell::new();
pub static SESSION_CONFIG: OnceCell<Arc<RwLock<SessionConfig>>> = OnceCell::new();


// Function to create a gRPC channel with TLS configuration
async fn create_channel(ip: &String) -> Result<Channel, Box<dyn std::error::Error>> {
    
    // Configure the TLS settings for the channel
    let tls_config = ClientTlsConfig::new()
        .ca_certificate(Certificate::from_pem(&CA_PUBLIC_CERTIFICATE))
        .domain_name(ip);

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

// Function to start the library and establish connections
pub fn start(ip: &String) -> Result<Receiver<Vec<u8>>, Box<dyn std::error::Error>> {
    // Set up a Tokio runtime
    let rt = Arc::new(Runtime::new().unwrap());

    // Create a session config to store the certificates and keys
    SESSION_CONFIG.set(Arc::new(RwLock::new(SessionConfig::default()))).unwrap();

    // Create EC Certificates  
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ec_key = EcKey::generate(&group).unwrap();

    // Run async code using the Tokio runtime, this will block until the connection is established
    rt.block_on(async {

        let arc_session_config = Arc::clone(&SESSION_CONFIG.get().unwrap());
        let mut interval = tokio::time::interval(Duration::from_secs(5));

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

                        let mut mutex_session_config = arc_session_config.write().unwrap();

                        mutex_session_config.client_ec_certificate = signed_cert;
                        mutex_session_config.client_ec_key = Some(ec_key);
                    }

                    // Create connection to retrive current Session encryption key and certificate to verify packets is comming from the server
                    {
                        let mut connection = KeyServiceClient::new(channel.clone());

                        let key_response = connection.request_current_key(RequestKey {}).await.unwrap();

                        let reponse = key_response.into_inner();

                        let mut mutex_session_config = arc_session_config.write().unwrap();

                        mutex_session_config.current_key = reponse.current_key.unwrap();
                        mutex_session_config.next_key = reponse.next_key.unwrap();
                        mutex_session_config.ec_server_certificate = reponse.certificate;
                        mutex_session_config.epoch = reponse.epoch;

                        info!("Current key: {:?}", mutex_session_config.current_key);
                        info!("Next key: {:?}", mutex_session_config.next_key);
                    }

                    // Create connection to retrieve current timestamp to base our client report intervals
                    {
                        let mut connection = NtsServiceClient::new(channel.clone());
                        
                        // Measure the time before the request
                        let start_time = Instant::now();
                        
                        let stamp_response = connection.get_current_timestamp(TimestampRequest {}).await.unwrap();
                        
                        // Measure the time after the request
                        let end_time = Instant::now();
                        
                        // Calculate the RTT
                        let rtt = end_time.duration_since(start_time);
                        
                        // Get the server timestamp
                        let timestamp_str = stamp_response.into_inner().timestamp;
                        let mut server_time = DateTime::parse_from_rfc3339(&timestamp_str).unwrap();
                        
                        // Substract half of the RTT to the server timestamp to compensate for the delay
                        server_time = server_time - chrono::Duration::from_std(rtt / 2).unwrap();
                        
                        TIME.set(Mutex::new(server_time.into())).unwrap();
                    }

                    // Save the client for use after the loop
                    CHANNEL.set(channel).unwrap();

                    break; // Exit the loop once connected
                }
                Err(e) => {
                    error!("Failed to connect: {}. Retrying in 5 seconds...", e);
                    interval.tick().await; // Wait for the next interval before retrying
                }
            }
        }
    });

    // If we arrive here we have certificates signed from PKI, Session Key and Current Timestamp

    // Create channels for thread comunication
    let (multicast_receiver_sender, multicast_receiver_receiver) = unbounded();
    let (unicast_receiver_sender, unicast_receiver_receiver) = unbounded();
    let (decoded_data_sender, decoded_data_receiver) = unbounded();
    let (client_report_sender, client_report_receiver) = unbounded();

    // Create the Client Session using the runtime that was init in the start
    let mut client_session = session::ClientSession::new(Arc::clone(&rt));

    // Creates new thread for multicast receiver
    // let multicast_receiver_handler = std::thread::spawn(move || {
    //     info!("Starting multicast receiver on address 239.255.0.1:12345.");
    //     receiver::multicast_receiver::server(
    //         "239.255.0.1".to_string(),
    //         "12345".to_string(),
    //         multicast_receiver_sender,
    //     ).unwrap();
    // });

    // Run the server task
    rt.spawn(async move {
        if let Err(e) = receiver::multicast_receiver::server("239.255.0.1".to_string(), "12345".to_string(), multicast_receiver_sender).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Run the unicast receiver task
    rt.spawn(async move {
        if let Err(e) = receiver::unicast_receiver::server(unicast_receiver_sender).await {
            eprintln!("Unicast receiver error: {}", e);
        }
    });


    // Dont need handler because when the listener goes out of scope the threads continue execution (infinite loops inside), i want the decoder to also go out of scope
    let decoder_handler = thread::spawn(move || {

        if let Err(e) = decoder::start(
            multicast_receiver_receiver,
            unicast_receiver_receiver,
            decoded_data_sender,
            client_report_sender
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
        decoder_handler.join().expect("decoder_handler panicked");
    });

    Ok(decoded_data_receiver)
}
