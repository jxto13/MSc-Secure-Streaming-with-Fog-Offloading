mod decoder;
pub mod receiver;
mod retransmission_manager;
mod session;
mod utils;

use crossbeam_channel::{unbounded, Receiver, Sender};
use iris_shared::compression::compress_data;
use prost::Message;

use crate::decoder::Decoder;
use crate::proto::mesh_packet::PacketType;
use crate::proto::{MeshPacket, RetransmissionRequest};
use crate::receiver::{Command, DynamicReceiver, ReceiverType};
use crate::retransmission_manager::RetransmissionManager;
use crate::session::ClientSession;

use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use proto::certificate_service_client::CertificateServiceClient;
use proto::key_service_client::KeyServiceClient;
use proto::nts_service_client::NtsServiceClient;
use proto::TimestampRequest;
use proto::{CertificateRequest, RequestKey};

use chrono::DateTime;
use log::{error, info};
use tokio::runtime::Runtime;
use tonic::transport::{Certificate, Channel, ClientTlsConfig};

pub mod proto {
    include!("../proto/generated/streaming_server.rs");
    include!("../proto/generated/edge_client.rs");
}

const CA_PUBLIC_CERTIFICATE: &[u8] = include_bytes!("../../server/certs/ca_certificate.pem");

/// Main client for managing streaming connections and data reception
pub struct Client {
    session: Arc<RwLock<ClientSession>>, // Shared session state
    channel: Option<Channel>,            // gRPC channel for communication with the server
    runtime: Arc<Runtime>,               // Tokio runtime for async operations

    retransmission_manager: Option<Arc<RetransmissionManager>>, // Manager for handling retransmissions with Mesh
    decoder: Option<Arc<Decoder>>, // Decoder for processing received data

    data_sender: Option<Sender<(ReceiverType, Vec<u8>)>>,
    command_sender: Option<crossbeam_channel::Sender<Command>>,
    buffer_sync_sender: Option<crossbeam_channel::Sender<Vec<crate::decoder::MatrixTracker>>>,

    thread_handles: RwLock<Vec<JoinHandle<()>>>,
    shutdown_flag: Arc<AtomicBool>,
}

impl Client {
    /// Create a new client instance
    pub fn new() -> Self {
        Self {
            session: Arc::new(RwLock::new(ClientSession::default())),
            channel: None,
            runtime: Arc::new(Runtime::new().expect("Failed to create Tokio runtime")),

            retransmission_manager: None,
            decoder: None,

            data_sender: None,
            command_sender: None,
            buffer_sync_sender: None,

            thread_handles: RwLock::new(Vec::new()),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the client and establish connections (blocking)
    /// Returns a receiver for decoded data
    pub fn start(&mut self, ip: &str) -> Result<Receiver<Vec<u8>>, Box<dyn std::error::Error>> {
        // Reset shutdown flag
        self.shutdown_flag.store(false, Ordering::Relaxed);

        // Create EC Certificates
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let ec_key = EcKey::generate(&group).unwrap();

        // Establish gRPC connection and setup session (blocking)
        let runtime = Arc::clone(&self.runtime);
        self.channel =
            Some(runtime.block_on(async { self.setup_grpc_connection(ip, ec_key).await })?);

        // Setup communication channels
        let (data_receiver_sender, data_receiver_receiver): (
            crossbeam_channel::Sender<(ReceiverType, Vec<u8>)>,
            crossbeam_channel::Receiver<(ReceiverType, Vec<u8>)>,
        ) = unbounded();
        let (decoded_data_sender, decoded_data_receiver) = unbounded();
        let (client_report_sender, client_report_receiver) = unbounded();
        let (command_sender, command_receiver) = unbounded();
        let (retransmission_manager_sender, retransmission_manager_receiver) = loole::unbounded();

        // Create buffer synchronization channel
        let (buffer_sync_sender, buffer_sync_receiver) = unbounded();
        self.buffer_sync_sender = Some(buffer_sync_sender);

        // Store command sender for socket management
        self.command_sender = Some(command_sender.clone());

        // Store data sender for receiving data
        self.data_sender = Some(data_receiver_sender.clone());

        // Start worker threads
        self.start_receiver_thread(data_receiver_sender.clone(), command_receiver)?;

        // Start the decoder thread
        self.start_decoder_thread(
            data_receiver_receiver,
            decoded_data_sender,
            client_report_sender,
            self.buffer_sync_sender.clone(),
        )?;

        // Start the client report thread
        self.start_report_thread(client_report_receiver, retransmission_manager_sender)?;

        // Create and start the retransmission manager with buffer sync
        self.retransmission_manager = Some(Arc::new(RetransmissionManager::new(
            Some(buffer_sync_receiver),
            retransmission_manager_receiver,
            data_receiver_sender.clone(),
        )));

        // Setup initial sockets
        self.setup_multicast_socket(&command_sender)?;
        self.setup_unicast_socket(&command_sender)?;

        info!("Client started successfully");
        Ok(decoded_data_receiver)
    }

    pub fn get_retransmission_manager(&self) -> Arc<RetransmissionManager> {
        self.retransmission_manager
            .as_ref()
            .expect("Retransmission manager not initialized")
            .clone()
    }

    /// Shutdown the client gracefully
    pub fn shutdown(&self) {
        info!("Shutting down client...");

        // Signal shutdown to all threads
        self.shutdown_flag.store(true, Ordering::Relaxed);

        // Join all threads
        let mut handles = self.thread_handles.write().unwrap();

        for handle in handles.drain(..) {
            if let Err(e) = handle.join() {
                error!("Error joining thread: {:?}", e);
            }
        }

        info!("Client shutdown complete");
    }

    /// Add an IPv6 link-local interface socket
    pub fn add_interface(&self, ipv6_interface: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Parse the IPv6 address and interface
        let parts: Vec<&str> = ipv6_interface.split('%').collect();
        if parts.len() != 2 {
            let error_msg = "Invalid IPv6 link-local format. Expected 'ipv6-address%interface'";
            log::error!("{}", error_msg);
            return Err(error_msg.into());
        }

        let ipv6_addr = parts[0].parse::<Ipv6Addr>().map_err(|e| {
            let error_msg = format!("Invalid IPv6 address '{}': {}", parts[0], e);
            log::error!("{}", error_msg);
            error_msg
        })?;

        let interface_name = parts[1];

        // Use pnet to get interface index
        let interfaces = pnet_datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .find(|iface| iface.name == interface_name)
            .ok_or_else(|| format!("Interface '{}' not found", interface_name))?;

        let scope_id = interface.index;

        // Verify it's a link-local address
        if !ipv6_addr.is_unicast_link_local() {
            let error_msg = "Address is not a link-local IPv6 address";
            log::error!("{}", error_msg);
            return Err(error_msg.into());
        }

        let port = self
            .session
            .read()
            .unwrap()
            .retransmission_port
            .try_into()
            .map_err(|_| {
                let error_msg = "Invalid port number for IPv6 socket";
                log::error!("{}", error_msg);
                error_msg
            })?;

        // Create the socket address
        let socket_addr = SocketAddrV6::new(ipv6_addr, port, 0, scope_id);

        // Create and bind the MIO UDP socket
        let socket = mio::net::UdpSocket::bind(SocketAddr::V6(socket_addr)).map_err(|e| {
            let error_msg = format!("Failed to bind IPv6 socket to {}: {}", socket_addr, e);
            log::error!("{}", error_msg);
            error_msg
        })?;

        // Get the command sender
        let command_sender = self.command_sender.as_ref().ok_or_else(|| {
            let error_msg = "Command sender not initialized. Call start first.";
            log::error!("{}", error_msg);
            error_msg
        })?;

        // Send the add socket command
        command_sender
            .send(Command::AddSocket(socket, ReceiverType::WifiAware))
            .map_err(|e| {
                let error_msg = format!("Failed to send add socket command: {}", e);
                log::error!("{}", error_msg);
                error_msg
            })?;

        info!(
            "Added IPv6 link-local socket for {} on port {}",
            ipv6_interface, port
        );
        Ok(())
    }

    /// Setup gRPC connection and session keys
    async fn setup_grpc_connection(
        &self,
        ip: &str,
        ec_key: EcKey<Private>,
    ) -> Result<Channel, Box<dyn std::error::Error>> {
        // Create channel
        let channel = self.create_channel(ip).await?;

        // Resolve IPv4 address
        if let Ok(addr) = self.resolve_ipv4_std(ip) {
            info!("Resolved IPv4 address: {}", addr);
            self.session.write().unwrap().server_ip = addr.to_string();
        }

        // Get client certificate
        self.setup_client_certificate(&channel, ec_key).await?;

        // Get session keys
        self.setup_session_keys(&channel).await?;

        // Get current timestamp
        self.setup_timestamp(&channel).await?;

        Ok(channel)
    }

    /// Create a gRPC channel to the server
    async fn create_channel(&self, ip: &str) -> Result<Channel, Box<dyn std::error::Error>> {
        let tls_config = ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(&CA_PUBLIC_CERTIFICATE))
            .domain_name(ip);

        let channel_url = format!("https://{}:50051", ip);

        let channel = Channel::builder(channel_url.parse()?)
            .tls_config(tls_config)?
            .keep_alive_timeout(Duration::from_secs(10))
            .keep_alive_while_idle(true)
            .http2_keep_alive_interval(Duration::from_secs(10))
            .keep_alive_timeout(Duration::from_secs(10))
            .connect()
            .await?;

        info!("Connected to GRPC Server at {}", ip);
        Ok(channel)
    }

    fn resolve_ipv4_std(&self, hostname: &str) -> Result<Ipv4Addr, Box<dyn std::error::Error>> {
        if hostname.trim().is_empty() {
            return Err("Hostname cannot be empty".into());
        }

        let port = 0;
        let addr_iter = (hostname, port).to_socket_addrs()?;

        for addr in addr_iter {
            if let std::net::IpAddr::V4(ipv4) = addr.ip() {
                return Ok(ipv4);
            }
        }

        Err(format!("No IPv4 address found for hostname '{}'", hostname).into())
    }

    async fn setup_client_certificate(
        &self,
        channel: &Channel,
        ec_key: EcKey<Private>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut connection = CertificateServiceClient::new(channel.clone());

        let pkey = PKey::from_ec_key(ec_key.clone()).unwrap();
        let request = utils::mk_request(&pkey).unwrap();
        let csr_vec = request.to_pem().unwrap();
        let csr_request = CertificateRequest { csr: csr_vec };

        let response = connection.sign_certificate(csr_request).await.unwrap();
        let signed_cert = response.into_inner().signed_cert;

        let mut session = self.session.write().unwrap();
        session.client_ec_certificate = signed_cert;
        session.client_ec_key = Some(ec_key);

        Ok(())
    }

    async fn setup_session_keys(
        &self,
        channel: &Channel,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut connection = KeyServiceClient::new(channel.clone());
        let key_response = connection.request_current_key(RequestKey {}).await.unwrap();
        let response = key_response.into_inner();

        let mut session = self.session.write().unwrap();
        session.current_key = response.current_key.unwrap();
        session.next_key = response.next_key.unwrap();
        session.ec_server_certificate = response.certificate;

        let biggy_back_data = response.biggy_back_data.unwrap();
        session.epoch = biggy_back_data.epoch;
        session.client_reporting_interval = biggy_back_data.client_reporting_interval;
        session.client_sliding_window = biggy_back_data.client_sliding_window;
        session.retransmission_port = biggy_back_data.retransmission_port;
        session.unicast_port = biggy_back_data.unicast_port;
        session.multicast_address = biggy_back_data.multicast_address;
        session.multicast_port = biggy_back_data.multicast_port;

        info!("Session keys configured successfully");
        Ok(())
    }

    async fn setup_timestamp(&self, channel: &Channel) -> Result<(), Box<dyn std::error::Error>> {
        let mut connection = NtsServiceClient::new(channel.clone());
        let start_time = Instant::now();
        let stamp_response = connection
            .get_current_timestamp(TimestampRequest {})
            .await
            .unwrap();
        let end_time = Instant::now();

        let rtt = end_time.duration_since(start_time);
        let timestamp_str = stamp_response.into_inner().timestamp;
        let mut server_time = DateTime::parse_from_rfc3339(&timestamp_str).unwrap();
        server_time = server_time - chrono::Duration::from_std(rtt / 2).unwrap();

        self.session.write().unwrap().server_time = server_time;
        info!("Server time synchronized: {}", server_time);
        Ok(())
    }

    fn start_receiver_thread(
        &mut self,
        data_receiver_sender: crossbeam_channel::Sender<(ReceiverType, Vec<u8>)>,
        command_receiver: crossbeam_channel::Receiver<Command>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let shutdown_flag = Arc::clone(&self.shutdown_flag);

        let handle = thread::spawn(move || {
            let server =
                DynamicReceiver::new(data_receiver_sender, command_receiver, shutdown_flag);

            let mut server = match server {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to create DynamicReceiver: {}", e);
                    return;
                }
            };

            if let Err(e) = server.run() {
                error!("Receiver server error: {}", e);
            }
            info!("Receiver thread shutting down");
        });

        self.thread_handles.write().unwrap().push(handle);
        Ok(())
    }

    fn start_decoder_thread(
        &mut self,
        data_receiver_receiver: crossbeam_channel::Receiver<(ReceiverType, Vec<u8>)>,
        decoded_data_sender: crossbeam_channel::Sender<Vec<u8>>,
        client_report_sender: crossbeam_channel::Sender<proto::ClientReport>,
        buffer_sync_sender: Option<crossbeam_channel::Sender<Vec<crate::decoder::MatrixTracker>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let shutdown_flag = Arc::clone(&self.shutdown_flag);
        let session = Arc::clone(&self.session);
        let channel = self.channel.as_ref().unwrap().clone();

        // Create packet request channel (still needed for backwards compatibility)
        let (_packet_request_sender, packet_request_receiver) = unbounded();

        let decoder = Arc::new(Decoder::new(
            data_receiver_receiver,
            decoded_data_sender,
            client_report_sender,
            packet_request_receiver,
            buffer_sync_sender,
            shutdown_flag.clone(),
            session,
            channel,
        ));

        self.decoder = Some(Arc::clone(&decoder));

        // Start the main decoder thread
        let handle = thread::spawn(move || {
            decoder.start();
            info!("Decoder thread shutting down");
        });

        self.thread_handles.write().unwrap().push(handle);
        Ok(())
    }

    fn start_report_thread(
        &mut self,
        client_report_receiver: crossbeam_channel::Receiver<proto::ClientReport>,
        retransmission_manager_sender: loole::Sender<MeshPacket>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let shutdown_flag = Arc::clone(&self.shutdown_flag);
        let session = Arc::clone(&self.session);

        let handle = thread::spawn(move || {
            // Get server configuration
            let (server_ip, retransmission_port) = {
                match session.read() {
                    Ok(session_config) => (
                        session_config.server_ip.clone(),
                        session_config.retransmission_port,
                    ),
                    Err(e) => {
                        error!("Failed to read session config in report thread: {}", e);
                        return;
                    }
                }
            };

            let report_sink_dst =
                match format!("{}:{}", server_ip, retransmission_port).parse::<SocketAddr>() {
                    Ok(addr) => addr,
                    Err(e) => {
                        error!("Failed to parse report sink address: {}", e);
                        return;
                    }
                };

            let socket = match std::net::UdpSocket::bind("0.0.0.0:0") {
                Ok(socket) => socket,
                Err(e) => {
                    error!("Couldn't bind to address for client reports: {}", e);
                    return;
                }
            };

            if let Ok(addr) = socket.local_addr() {
                info!("Client report socket bound to: {}", addr);
            }

            for report in client_report_receiver.iter() {
                if shutdown_flag.load(Ordering::Relaxed) {
                    info!("Client report thread shutting down gracefully");
                    break;
                }

                // Create a MeshPacket for retransmission per packet loss
                for item in &report.reports {
                    let indexes = Client::find_zero_indices(&item.bitmap);

                    for index in indexes {
                        let packet = MeshPacket {
                            ttl: 3,
                            packet_type: Some(PacketType::Request(RetransmissionRequest {
                                matrix: item.matrix_index as i32,
                                packet: index as i32,
                            })),
                        };

                        // Send the packet to the retransmission manager
                        // This will handle the retransmission logic
                        if let Err(e) = retransmission_manager_sender.send(packet.clone()) {
                            error!("Failed to send retransmission request: {}", e);
                        }
                    }
                }

                let compressed = compress_data(&report.encode_to_vec());
                if let Err(e) = socket.send_to(&compressed, &report_sink_dst) {
                    error!("Failed to send UDP message: {}", e);
                }
            }
            info!("Report thread shutting down");
        });

        self.thread_handles.write().unwrap().push(handle);
        Ok(())
    }

    fn setup_multicast_socket(
        &self,
        command_sender: &crossbeam_channel::Sender<Command>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let session = self.session.read().unwrap();
        let multicast_ip = session.multicast_address.clone();
        let multicast_port = session.multicast_port;
        drop(session);

        let multicast_addr =
            format!("{}:{}", multicast_ip, multicast_port).parse::<SocketAddr>()?;
        let multicast_socket = mio::net::UdpSocket::bind(multicast_addr)?;
        let multicast_ip_parsed = multicast_ip.parse::<Ipv4Addr>()?;

        multicast_socket.join_multicast_v4(&multicast_ip_parsed, &Ipv4Addr::UNSPECIFIED)?;
        command_sender.send(Command::AddSocket(
            multicast_socket,
            ReceiverType::Multicast,
        ))?;

        info!("Multicast socket setup complete");
        Ok(())
    }

    fn setup_unicast_socket(
        &self,
        command_sender: &crossbeam_channel::Sender<Command>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let unicast_port = self.session.read().unwrap().unicast_port;
        let unicast_addr = format!("0.0.0.0:{}", unicast_port).parse::<SocketAddr>()?;
        let unicast_socket = mio::net::UdpSocket::bind(unicast_addr)?;

        command_sender.send(Command::AddSocket(unicast_socket, ReceiverType::Unicast))?;
        info!("Unicast socket bound to: {}", unicast_addr);
        Ok(())
    }

    /// Returns all indices where the bit is `0` in the bitmap.
    pub fn find_zero_indices(vec: &Vec<u8>) -> Vec<usize> {
        vec.iter()
            .enumerate()
            .filter_map(|(index, &value)| if value == 0 { Some(index) } else { None })
            .collect()
    }

    pub fn data_sender(&self) -> Option<Sender<(ReceiverType, Vec<u8>)>> {
        self.data_sender.clone()
    }
    /// Returns the most recent packet loss percentage (0-100)
    pub fn get_packet_loss_percentage(&self) -> f64 {
        match &self.decoder {
            Some(decoder) => {
                let metrics = decoder.get_metrics();
                let metrics = metrics.read().unwrap();
                metrics.packet_loss_percentage
            }
            None => {
                error!("Decoder is not initialized");
                return 0.0;
            }
        }
    }
}
