mod decoder;
mod receiver;
mod session;
mod utils;

use crossbeam_channel::{unbounded, Receiver};

use iris_shared::compression::compress_data;
use once_cell::sync::OnceCell;
use prost::Message;

use crate::receiver::{Command, DynamicReceiver, ReceiverType};
use crate::session::ClientSession;

use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::PKey;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use proto::certificate_service_client::CertificateServiceClient;
use proto::key_service_client::KeyServiceClient;
use proto::nts_service_client::NtsServiceClient;
use proto::TimestampRequest;
use proto::{CertificateRequest, RequestKey};
use std::net::ToSocketAddrs;

use tokio::{self};

use tokio::runtime::Runtime;

use tonic::transport::{Certificate, Channel, ClientTlsConfig};

use chrono::DateTime;

use log::{error, info};

pub mod proto {
    include!("../proto/generated/streaming_server.rs");
    include!("../proto/generated/edge_client.rs");
}

const CA_PUBLIC_CERTIFICATE: &[u8] = include_bytes!("../../server/certs/ca_certificate.pem");

pub static CHANNEL: OnceCell<Arc<Channel>> = OnceCell::new();
pub static SESSION: OnceCell<Arc<RwLock<ClientSession>>> = OnceCell::new();

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
        .keep_alive_timeout(Duration::from_secs(10))
        .keep_alive_while_idle(true)
        .http2_keep_alive_interval(Duration::from_secs(10))
        .keep_alive_timeout(Duration::from_secs(10))
        .connect()
        .await?;

    info!("Connected to GRPC Server at {}", ip);

    // If we reach here, connection was successful
    Ok(channel)
}

fn resolve_ipv4_std(hostname: &str) -> Result<Ipv4Addr, Box<dyn std::error::Error>> {
    if hostname.trim().is_empty() {
        return Err("Hostname cannot be empty".into());
    }

    // Use dummy port 0 just for resolution
    let port = 0;
    let addrs = (hostname, port).to_socket_addrs();

    let addr_iter = match addrs {
        Ok(iter) => iter,
        Err(e) => {
            return Err(format!("Failed to resolve hostname '{}': {}", hostname, e).into());
        }
    };

    // Look for the first IPv4 address
    for addr in addr_iter {
        match addr.ip() {
            std::net::IpAddr::V4(ipv4) => return Ok(ipv4),
            std::net::IpAddr::V6(_) => continue, // Skip IPv6
        }
    }

    Err(format!("No IPv4 address found for hostname '{}'", hostname).into())
}

// Function to start the library and establish connections
pub fn start_client(ip: &String) -> Result<Receiver<Vec<u8>>, Box<dyn std::error::Error>> {
    // Set up a Tokio runtime
    let rt = Arc::new(Runtime::new().unwrap());

    // Create a session config to store the certificates and keys
    SESSION
        .set(Arc::new(RwLock::new(ClientSession::default())))
        .unwrap();

    // Create EC Certificates
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let ec_key = EcKey::generate(&group).unwrap();

    // Run async code using the Tokio runtime, this will block until the connection is established
    rt.block_on(async {
        let arc_session_config = Arc::clone(&SESSION.get().unwrap());
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            match create_channel(ip).await {
                Ok(channel) => {
                    // If we reach here, we have a valid channel to the server

                    // Resolve the IPv4 address
                    let ipv4 = resolve_ipv4_std(ip)
                        .map_err(|e| format!("Failed to resolve IPv4 address: {}", e));

                    match ipv4 {
                        Ok(addr) => {
                            info!("Resolved IPv4 address: {}", addr);
                            // Store the resolved IPv4 address in a static variable
                            let mut mutex_session_config = arc_session_config.write().unwrap();
                            mutex_session_config.server_ip = addr.to_string();
                        }
                        Err(e) => {
                            error!("Error resolving IPv4 address: {}", e);
                            continue; // Skip this iteration and retry
                        }
                    }

                    info!("Successfully connected to the gRPC server at {}", ip);

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
                        let response = connection.sign_certificate(csr_request).await.unwrap();
                        let signed_cert = response.into_inner().signed_cert;

                        let mut mutex_session_config = arc_session_config.write().unwrap();

                        mutex_session_config.client_ec_certificate = signed_cert;
                        mutex_session_config.client_ec_key = Some(ec_key);
                    }

                    // Create connection to retrive current Session encryption key and certificate to verify packets is comming from the server
                    {
                        let mut connection = KeyServiceClient::new(channel.clone());

                        let key_response =
                            connection.request_current_key(RequestKey {}).await.unwrap();

                        let reponse = key_response.into_inner();

                        let mut mutex_session_config = arc_session_config.write().unwrap();

                        mutex_session_config.current_key = reponse.current_key.unwrap();
                        mutex_session_config.next_key = reponse.next_key.unwrap();
                        mutex_session_config.ec_server_certificate = reponse.certificate;

                        // If the server provided biggy back data, update the session config
                        let biggy_back_data = reponse.biggy_back_data.unwrap();

                        mutex_session_config.epoch = biggy_back_data.epoch;
                        mutex_session_config.client_reporting_interval =
                            biggy_back_data.client_reporting_interval;
                        mutex_session_config.client_sliding_window =
                            biggy_back_data.client_sliding_window;
                        mutex_session_config.retransmission_port =
                            biggy_back_data.retransmission_port;
                        mutex_session_config.unicast_port = biggy_back_data.unicast_port;
                        mutex_session_config.multicast_address = biggy_back_data.multicast_address;
                        mutex_session_config.multicast_port = biggy_back_data.multicast_port;

                        info!("Current key: {:?}", mutex_session_config.current_key);
                        info!("Next key: {:?}", mutex_session_config.next_key);
                    }

                    // Create connection to retrieve current timestamp to base our client report intervals
                    {
                        let mut connection = NtsServiceClient::new(channel.clone());

                        // Measure the time before the request
                        let start_time = Instant::now();

                        let stamp_response = connection
                            .get_current_timestamp(TimestampRequest {})
                            .await
                            .unwrap();

                        // Measure the time after the request
                        let end_time = Instant::now();

                        // Calculate the RTT
                        let rtt = end_time.duration_since(start_time);

                        // Get the server timestamp
                        let timestamp_str = stamp_response.into_inner().timestamp;
                        let mut server_time = DateTime::parse_from_rfc3339(&timestamp_str).unwrap();

                        // Substract half of the RTT to the server timestamp to compensate for the delay
                        server_time = server_time - chrono::Duration::from_std(rtt / 2).unwrap();

                        let mut mutex_session_config = arc_session_config.write().unwrap();
                        mutex_session_config.server_time = server_time;

                        info!("Server time: {}", server_time);
                    }

                    // Save the client for use after the loop
                    CHANNEL.set(Arc::new(channel)).unwrap();

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
    let (data_receiver_sender, data_receiver_receiver) = unbounded();
    let (decoded_data_sender, decoded_data_receiver) = unbounded();
    let (client_report_sender, client_report_receiver) = unbounded();
    let (command_sender, command_receiver) = unbounded();

    // Create the Client Session using the runtime that was init in the start
    // let mut client_session = session::ClientSession::new(Arc::clone(&rt));

    // Spawn Receiver Thread
    let receiver_handler = thread::spawn(move || {
        let server = DynamicReceiver::new(data_receiver_sender, command_receiver);

        // Handle any error that might occur during server initialization
        let mut server = match server {
            Ok(s) => s,
            Err(e) => {
                panic!("Failed to create DynamicReceiver: {}", e);
            }
        };
        // Run the server event loop
        server.run().unwrap();
    });

    // REMEMBER TO GET THE IP FROM THE CONFIGURATION FILE OF GRPC SERVER
    let multicast_ip = SESSION
        .get()
        .unwrap()
        .read()
        .unwrap()
        .multicast_address
        .clone();
    let multicast_port = SESSION.get().unwrap().read().unwrap().multicast_port;

    // Setup multicast socket
    {
        let multicast_addr = SocketAddr::new(multicast_ip.parse().unwrap(), multicast_port as u16);
        let multicast_socket = mio::net::UdpSocket::bind(multicast_addr).unwrap();
        multicast_socket
            .join_multicast_v4(&multicast_ip.parse().unwrap(), &Ipv4Addr::UNSPECIFIED)
            .unwrap();
        command_sender
            .send(Command::AddSocket(
                multicast_socket,
                ReceiverType::Multicast,
            ))
            .unwrap();
    }

    // Setup unicast socket
    {
        // Use the unicast port from the session configuration
        let unicast_port = SESSION.get().unwrap().read().unwrap().unicast_port;
        // Bind to the unicast address and port
        let unicast_addr = format!("0.0.0.0:{}", unicast_port).parse().unwrap();
        info!("Unicast socket bound to: {}", unicast_addr);
        let unicast_socket = mio::net::UdpSocket::bind(unicast_addr).unwrap();
        command_sender
            .send(Command::AddSocket(unicast_socket, ReceiverType::Unicast))
            .unwrap();
    }

    // Dont need handler because when the listener goes out of scope the threads continue execution (infinite loops inside), i want the decoder to also go out of scope
    let decoder_handler = thread::spawn(move || {
        decoder::start(
            data_receiver_receiver,
            decoded_data_sender,
            client_report_sender,
        );
    });

    // Spawn Client Report Thread
    let report_handler = thread::spawn(move || {
        // Get the multicast port from the configuration.
        let server_ip = SESSION.get().unwrap().read().unwrap().server_ip.clone();
        let retransmission_port = SESSION.get().unwrap().read().unwrap().retransmission_port;

        // Create the destination address for the report sink
        let report_sink_dst = format!("{}:{}", server_ip, retransmission_port)
            .parse::<SocketAddr>()
            .unwrap();

        // Bind to any available port
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind to address");
        info!(
            "Client report socket bound to: {}",
            socket.local_addr().unwrap()
        );

        for report in client_report_receiver.iter() {
            let compressed = compress_data(&report.encode_to_vec());

            // Use a reference to avoid moving report_sink_dst
            socket
                .send_to(&compressed, &report_sink_dst)
                .expect("Failed to send UDP message");
        }
    });

    // Spawn a separate thread for joining and returning the sender
    thread::spawn(move || {
        // Join all the threads to ensure they complete before returning
        receiver_handler.join().expect("receiver_handler panicked");
        decoder_handler.join().expect("decoder_handler panicked");
        report_handler.join().expect("client_session panicked");
    });

    Ok(decoded_data_receiver)
}
