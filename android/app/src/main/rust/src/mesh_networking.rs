use iris_client::proto::mesh_packet::PacketType;
use iris_client::proto::{HeartbeatPacket, MeshPacket};
use log::info;
use pnet_datalink;
use prost::Message;

use socket2::{Domain, Protocol, Socket, Type};
use std::collections::{HashMap, HashSet};
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio::time::interval;
use uuid::Uuid;

fn get_interface_index(name: &str) -> std::io::Result<u32> {
    let interfaces = pnet_datalink::interfaces();

    for iface in interfaces {
        if iface.name == name {
            return Ok(iface.index);
        }
    }

    let msg = format!("Interface '{}' not found", name);
    log::error!("{}", msg);
    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, msg))
}

fn get_local_interface_from_client_ipv6(name: &str) -> std::io::Result<(Ipv6Addr, u32)> {
    let interfaces = pnet_datalink::interfaces();

    for iface in interfaces {
        if iface.name == name {
            info!("Found interface: {} with index {}", iface.name, iface.index);
            // Find first IPv6 address (skip IPv4)
            for ip_net in &iface.ips {
                if let std::net::IpAddr::V6(ipv6) = ip_net.ip() {
                    return Ok((ipv6, iface.index));
                }
            }
        }
    }

    let msg = format!("Interface '{}' not found", name);
    log::error!("{}", msg);
    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, msg))
}

// === Shared types ===
pub type SeenSet = Arc<RwLock<HashSet<String>>>;
pub type SocketMap = Arc<RwLock<HashMap<String, UdpSocket>>>;
pub type PeerMap = Arc<Mutex<HashMap<String, Instant>>>;

// === Node Struct ===
pub struct Node {
    pub node_id: String,
    pub seen: SeenSet,
    pub peer_map: PeerMap,
    pub known_peers: Arc<RwLock<HashSet<String>>>,
    pub sockets: Arc<RwLock<HashMap<String, Arc<UdpSocket>>>>,
    pub socket_notify: tokio::sync::watch::Sender<()>,
    runtime: Arc<Runtime>,
}

impl Node {
    pub fn new() -> Arc<Self> {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        // Create a channel to notify about socket changes
        let (tx, _rx) = tokio::sync::watch::channel(());

        let node = Arc::new(Node {
            node_id: Uuid::new_v4().to_string(),
            seen: Arc::new(RwLock::new(HashSet::new())),
            peer_map: Arc::new(Mutex::new(HashMap::new())),
            known_peers: Arc::new(RwLock::new(HashSet::new())),
            sockets: Arc::new(RwLock::new(HashMap::new())),
            socket_notify: tx,
            runtime: Arc::new(runtime),
        });

        info!("Node {} initialized with Tokio runtime", node.node_id);

        // Spawn async tasks inside the runtime
        let node_clone = node.clone();
        node.runtime.spawn(async move {
            // Start the listen_and_relay task which internally spawns heartbeat tasks
            if let Err(e) = node_clone.listen_and_relay().await {
                eprintln!("listen_and_relay exited with error: {}", e);
            }
        });

        node
    }

    async fn create_reusable_udp_socket(addr: SocketAddrV6) -> std::io::Result<UdpSocket> {
        let domain = Domain::IPV6;
        let socket_type = Type::DGRAM;
        let protocol = Some(Protocol::UDP);

        let socket = Socket::new(domain, socket_type, protocol)?;

        socket.set_reuse_address(true)?;

        socket.bind(&addr.into())?;

        // Convert to tokio UdpSocket
        let std_socket: std::net::UdpSocket = socket.into();
        std_socket.set_nonblocking(true)?;
        UdpSocket::from_std(std_socket)
    }

    /// Synchronously adds a socket to the node.
    pub fn add_socket_sync(&self, addr: &str) -> std::io::Result<()> {
        self.runtime.block_on(self.add_socket(addr))
    }

    /// Parses an IPv6 address in the format "ipv6-address%interface" to a SocketAddrV6.
    fn parse_ipv6_str(addr: &str) -> std::io::Result<SocketAddrV6> {
        // Parse the IPv6 address and interface
        let parts: Vec<&str> = addr.split('%').collect();

        if parts.len() != 2 {
            let error_msg = "Invalid IPv6 link-local format. Expected 'ipv6-address%interface'";
            log::error!("{}", error_msg);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error_msg,
            ));
        }

        // Parse the IPv6 address
        let ipv6_addr = parts[0].parse::<Ipv6Addr>().map_err(|e| {
            let msg = format!("Invalid IPv6 address '{}': {}", parts[0], e);
            log::error!("{}", msg);
            std::io::Error::new(std::io::ErrorKind::InvalidInput, msg)
        })?;

        // Verify it's a link-local address
        if !ipv6_addr.is_unicast_link_local() {
            let error_msg = "Address is not a link-local IPv6 address";
            log::error!("{}", error_msg);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error_msg,
            ));
        }

        // Parse the interface scope ID
        let scope_id = get_interface_index(parts[1])?;

        Ok(SocketAddrV6::new(ipv6_addr, 9060, 0, scope_id))
    }

    /// Asynchronously adds a socket to the node.
    pub async fn add_socket(&self, addr: &str) -> std::io::Result<()> {
        // Create the socket address for local interface binding
        let local_ipv6_str = addr.split('%').nth(1).unwrap_or("::");

        let local_socket = match get_local_interface_from_client_ipv6(local_ipv6_str) {
            Ok((ipv6, scope_id)) => SocketAddrV6::new(ipv6, 9060, 0, scope_id),
            Err(e) => {
                log::error!("Failed to get local interface: {}", e);
                return Err(e);
            }
        };

        let socket = Self::create_reusable_udp_socket(local_socket)
            .await
            .map_err(|e| {
                log::error!("Failed to create reusable UDP socket: {}", e);
                e
            })?;

        let socket = Arc::new(socket);
        let mut map = self.sockets.write().await;

        // Insert the socket into the map using the string representation of the address
        map.insert(addr.to_owned(), socket);
        let _ = self.socket_notify.send(());
        Ok(())
    }

    /// Synchronously removes a socket from the node.
    pub fn remove_socket_sync(&self, addr: &str) {
        self.runtime.block_on(self.remove_socket(addr));
    }

    /// Asynchronously removes a socket from the node.
    pub async fn remove_socket(&self, addr: &str) {
        let mut map = self.sockets.write().await;
        map.remove(addr);
        let _ = self.socket_notify.send(());
    }

    pub async fn heartbeat_sender(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            let packet = MeshPacket {
                message_id: format!("heartbeat-{}", self.node_id),
                ttl: 0,
                packet_type: Some(PacketType::Heartbeat(HeartbeatPacket {
                    uuid: self.node_id.clone(),
                })),
            };
            let mut buf = Vec::new();
            if packet.encode(&mut buf).is_ok() {
                let sockets = self.sockets.read().await;
                for (sock_addr, socket) in sockets.iter() {
                    // Parse socket addr string once outside await points
                    let mut ipv6_addr = match Self::parse_ipv6_str(&sock_addr) {
                        Ok(addr) => addr,
                        Err(e) => {
                            eprintln!("Failed to parse socket addr: {}", e);
                            continue;
                        }
                    };

                    ipv6_addr.set_port(9060);

                    if let Err(e) = socket.send_to(&buf, ipv6_addr).await {
                        eprintln!("Failed to send ack: {}", e);
                    }
                }
            }
        }
    }

    pub async fn heartbeat_monitor(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(3));
        loop {
            interval.tick().await;
            let now = Instant::now();
            let mut map = self.peer_map.lock().unwrap();
            map.retain(|_, &mut last_seen| now.duration_since(last_seen) < Duration::from_secs(10));
            let active: Vec<String> = map.keys().cloned().collect();
            info!("[{}] Active peers: {:?}", self.node_id, active);
        }
    }

    pub async fn listen_and_relay(self: Arc<Self>) -> std::io::Result<()> {
        use futures::{stream::FuturesUnordered, StreamExt};
        use tokio::select;

        // --- Spawn heartbeat tasks once ---
        let heartbeat_node = self.clone();
        tokio::spawn(async move {
            heartbeat_node.heartbeat_sender().await;
        });

        let monitor_node = self.clone();
        tokio::spawn(async move {
            monitor_node.heartbeat_monitor().await;
        });

        // --- Socket change listener ---
        let mut rx = self.socket_notify.subscribe();

        loop {
            // Clone current sockets into a snapshot list
            let sockets_snapshot = {
                let sockets = self.sockets.read().await;
                sockets
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>()
            };

            if sockets_snapshot.is_empty() {
                info!(
                    "[{}] No sockets available, waiting for changes...",
                    self.node_id
                );
                rx.changed().await.ok();
                continue;
            }

            // Create FuturesUnordered of recv_from calls
            let mut recv_stream: FuturesUnordered<_> = sockets_snapshot
                .iter()
                .map(|(key, socket)| {
                    let mut buf = vec![0u8; 1024];
                    let socket = socket.clone();
                    let key = key.clone();
                    async move {
                        match socket.recv_from(&mut buf).await {
                            Ok((len, addr)) => Some((key, socket, addr, buf[..len].to_vec())),
                            Err(e) => {
                                log::error!("Receive error on {}: {}", key, e);
                                None
                            }
                        }
                    }
                })
                .collect();

            // Wait for socket input or change notification
            select! {
                _ = rx.changed() => {
                    info!("[{}] Socket change detected, reloading recv tasks...", self.node_id);
                    continue;
                }

                maybe_packet = recv_stream.next() => {
                    if let Some(Some((sock_addr, socket, sender_addr, data))) = maybe_packet {
                        self.peer_map.lock().unwrap().insert(sender_addr.to_string(), Instant::now());

                        if let Ok(packet) = MeshPacket::decode(&data[..]) {

                            // Process the packet based on its type (clone to avoid borrow issues and forward it )
                            if let Some(packet_type) = packet.packet_type.clone() {
                                match packet_type {
                                    PacketType::Heartbeat(heartbeat) => {
                                        let mut peers = self.known_peers.write().await;
                                        // Only add peer if sender's node_id is not self
                                        if heartbeat.uuid != self.node_id && !peers.contains(&heartbeat.uuid) {
                                            peers.insert(heartbeat.uuid.clone());
                                            info!("[{}] Learned new peer: {} at {}", self.node_id, heartbeat.uuid, sender_addr);
                                        }
                                        continue;
                                    }
                                    PacketType::Request(request) => {
                                        info!("Received retransmission request: {:?}", request);
                                    }
                                    PacketType::Response(response) => {
                                        info!("Received retransmission response: {:?}", response);
                                    }
                                }
                            }

                            if packet.ttl <= 0 || !self.should_process(&packet.message_id).await {
                                continue;
                            }

                            info!(
                                "[{}:{}] Received from {}: {}",
                                self.node_id, sock_addr, sender_addr, packet.message_id
                            );

                            let new_packet = MeshPacket {
                                ttl: packet.ttl - 1,
                                ..packet
                            };

                            let mut encoded = Vec::new();
                            new_packet.encode(&mut encoded).unwrap();

                            let peers = self.known_peers.read().await;
                            for peer in peers.iter() {
                                if let Ok(target) = peer.parse::<std::net::SocketAddr>() {
                                    if target != sender_addr {
                                        if let Err(e) = socket.send_to(&encoded, target).await {
                                            log::error!("Relay send error to {}: {}", target, e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn should_process(&self, message_id: &str) -> bool {
        let mut set = self.seen.write().await;
        if set.contains(message_id) {
            false
        } else {
            set.insert(message_id.to_string());
            true
        }
    }
}
