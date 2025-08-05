use iris_client::proto::mesh_packet::PacketType;
use iris_client::proto::{
    HeartbeatPacket, MeshPacket, RetransmissionRequest, RetransmissionResponse,
};
use log::{debug, error, info, warn};
use pnet_datalink;
use prost::Message;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::{HashMap, HashSet};
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, watch, RwLock};
use tokio::time::interval;
use uuid::Uuid;

// === Routing and Shared Types ===
#[derive(Debug, Clone)]
pub struct RoutingEntry {
    pub next_hop: SocketAddr,
    pub cost: u32,
    pub last_seen: Instant,
}

pub type RoutingTable = Arc<RwLock<HashMap<String, RoutingEntry>>>;
pub type SeenSet = Arc<RwLock<HashSet<String>>>;
pub type PeerMap = Arc<Mutex<HashMap<String, Instant>>>;

// Channels for application-level request/response injection
pub struct AppChannels {
    pub request_from_mesh: (loole::Sender<MeshPacket>, loole::Receiver<MeshPacket>),
    pub request_to_mesh: (loole::Sender<MeshPacket>, loole::Receiver<MeshPacket>),
}

pub struct Node {
    pub node_id: String,
    pub seen: SeenSet,
    pub peer_map: PeerMap,
    pub routing: RoutingTable,
    pub sockets: Arc<RwLock<HashMap<String, Arc<UdpSocket>>>>,
    pub socket_notify: watch::Sender<()>,
    pub channels: Arc<AppChannels>,
    pub runtime: Arc<Runtime>,
    pub heartbeat_counter: Arc<tokio::sync::Mutex<u64>>, // Counter for heartbeat messages
}

impl Node {
    pub fn new() -> Arc<Self> {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        let (tx_notify, _) = watch::channel(());

        let node = Arc::new(Self {
            node_id: Uuid::new_v4().to_string(),
            seen: Arc::new(RwLock::new(HashSet::new())),
            peer_map: Arc::new(Mutex::new(HashMap::new())),
            routing: Arc::new(RwLock::new(HashMap::new())),
            sockets: Arc::new(RwLock::new(HashMap::new())),
            socket_notify: tx_notify,
            channels: Arc::new(AppChannels {
                request_from_mesh: loole::unbounded(),
                request_to_mesh: loole::unbounded(),
            }),
            runtime: Arc::new(runtime),
            heartbeat_counter: Arc::new(tokio::sync::Mutex::new(0)),
        });

        info!("Node {} initialized", node.node_id);

        // Launch tasks
        let node_clone = node.clone();
        node.runtime.spawn(async move {
            if let Err(e) = node_clone.listen_and_relay().await {
                error!("listen_and_relay exited with error: {}", e);
            }
        });

        // Route pruning
        let routing = node.routing.clone();
        node.runtime.spawn(async move {
            Node::prune_routes(routing).await;
        });

        // Peer cleanup
        let peer_map = node.peer_map.clone();
        node.runtime.spawn(async move {
            Node::cleanup_peers(peer_map).await;
        });

        node
    }

    pub fn get_ipc_channels(&self) -> Arc<AppChannels> {
        self.channels.clone()
    }

    // === Interface helpers ===
    fn get_interface_index(name: &str) -> std::io::Result<u32> {
        for iface in pnet_datalink::interfaces() {
            if iface.name == name {
                return Ok(iface.index);
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Interface '{}' not found", name),
        ))
    }

    fn get_local_interface_from_client_ipv6(name: &str) -> std::io::Result<(Ipv6Addr, u32)> {
        for iface in pnet_datalink::interfaces() {
            if iface.name == name {
                for ip_net in &iface.ips {
                    if let std::net::IpAddr::V6(ipv6) = ip_net.ip() {
                        return Ok((ipv6, iface.index));
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Interface '{}' not found", name),
        ))
    }

    fn parse_ipv6_str(addr: &str) -> std::io::Result<SocketAddrV6> {
        let parts: Vec<&str> = addr.split('%').collect();
        if parts.len() != 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid IPv6 link-local format",
            ));
        }
        let ipv6_addr = parts[0]
            .parse::<Ipv6Addr>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        if !ipv6_addr.is_unicast_link_local() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Address is not link-local IPv6",
            ));
        }
        let scope_id = Self::get_interface_index(parts[1])?;
        Ok(SocketAddrV6::new(ipv6_addr, 9060, 0, scope_id))
    }

    async fn create_reusable_udp_socket(addr: SocketAddrV6) -> std::io::Result<UdpSocket> {
        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        socket.bind(&addr.into())?;
        let std_socket: std::net::UdpSocket = socket.into();
        std_socket.set_nonblocking(true)?;
        Ok(UdpSocket::from_std(std_socket)?)
    }

    pub fn add_socket_sync(&self, addr: &str) -> std::io::Result<()> {
        self.runtime.block_on(self.add_socket(addr))
    }

    pub async fn add_socket(&self, addr: &str) -> std::io::Result<()> {
        let iface_name = addr.split('%').nth(1).unwrap_or("::");
        let local_socket = match Self::get_local_interface_from_client_ipv6(iface_name) {
            Ok((ipv6, scope_id)) => SocketAddrV6::new(ipv6, 9060, 0, scope_id),
            Err(e) => return Err(e),
        };
        let socket = Arc::new(Self::create_reusable_udp_socket(local_socket).await?);
        let mut map = self.sockets.write().await;
        map.insert(addr.to_string(), socket);
        let _ = self.socket_notify.send(());
        Ok(())
    }

    pub fn remove_socket_sync(&self, addr: &str) {
        self.runtime.block_on(self.remove_socket(addr));
    }

    pub async fn remove_socket(&self, addr: &str) {
        self.sockets.write().await.remove(addr);
        let _ = self.socket_notify.send(());
    }

    // === Heartbeat + Routing ===
    async fn heartbeat_sender(self: Arc<Self>) {
        use tokio::select;
        let mut ticker = interval(Duration::from_secs(10));
        let mut rx = self.socket_notify.subscribe();
        loop {
            select! {
                _ = ticker.tick() => {
                    let routing = self.routing.read().await;
                    let peer_list: Vec<String> = routing.keys().cloned().collect();

                    // Increment heartbeat counter
                    let mut counter = self.heartbeat_counter.lock().await;
                    *counter += 1;
                    let current_count = *counter;

                    let hb = MeshPacket {
                        message_id: format!("heartbeat-{}-{}", self.node_id, current_count),
                        ttl: 0,
                        packet_type: Some(PacketType::Heartbeat(HeartbeatPacket {
                            uuid: self.node_id.clone(),
                            addr: "".into(),
                            peers: peer_list,
                        })),
                    };
                    let mut buf = Vec::new();
                    hb.encode(&mut buf).unwrap();

                    let sockets = self.sockets.read().await;
                    for (sock_addr, socket) in sockets.iter() {
                        if let Ok(mut ipv6) = Self::parse_ipv6_str(sock_addr) {
                            ipv6.set_port(9060);
                            let res = socket.send_to(&buf, ipv6).await;
                            if let Err(e) = res {
                                debug!("Failed to send heartbeat to {}: {}", sock_addr, e);
                            }
                        } else {
                            debug!("Invalid IPv6 address for socket: {}", sock_addr);
                        }
                    }
                }
                _ = rx.changed() => {
                    debug!("Socket list changed, heartbeat sender will use updated sockets.");
                }
            }
        }
    }

    async fn update_routing(&self, hb: &HeartbeatPacket, sender_addr: SocketAddr) {
        let mut routing = self.routing.write().await;

        routing.insert(
            hb.uuid.clone(),
            RoutingEntry {
                next_hop: sender_addr,
                cost: 1,
                last_seen: Instant::now(),
            },
        );

        for peer in &hb.peers {
            if peer != &self.node_id && !routing.contains_key(peer) {
                routing.insert(
                    peer.clone(),
                    RoutingEntry {
                        next_hop: sender_addr,
                        cost: 2,
                        last_seen: Instant::now(),
                    },
                );
            }
        }
    }

    async fn prune_routes(routing: RoutingTable) {
        let mut ticker = interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            routing
                .write()
                .await
                .retain(|_, entry| entry.last_seen.elapsed() < Duration::from_secs(30));
        }
    }

    async fn send_to_peer(&self, dest_uuid: &str, packet: &MeshPacket) -> std::io::Result<()> {
        let routing = self.routing.read().await;
        if let Some(route) = routing.get(dest_uuid) {
            let socket = {
                let sockets = self.sockets.read().await;
                sockets.values().next().unwrap().clone()
            };
            let mut buf = Vec::new();
            packet.encode(&mut buf).unwrap();
            socket.send_to(&buf, route.next_hop).await?;
            Ok(())
        } else {
            warn!("No route to {}, broadcasting", dest_uuid);
            self.broadcast(packet).await
        }
    }

    async fn broadcast(&self, packet: &MeshPacket) -> std::io::Result<()> {
        let mut buf = Vec::new();
        packet.encode(&mut buf).unwrap();
        let sockets = self.sockets.read().await;
        for (sock_addr, socket) in sockets.iter() {
            if let Ok(mut ipv6) = Self::parse_ipv6_str(sock_addr) {
                ipv6.set_port(9060);
                socket.send_to(&buf, ipv6).await?;
            }
        }
        Ok(())
    }

    // === Core Listener ===
    pub async fn listen_and_relay(self: Arc<Self>) -> std::io::Result<()> {
        use futures::{stream::FuturesUnordered, StreamExt};
        use tokio::select;

        tokio::spawn(self.clone().heartbeat_sender());

        let mut rx = self.socket_notify.subscribe();
        loop {
            let sockets_snapshot: Vec<_> = {
                let sockets = self.sockets.read().await;
                sockets
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            };

            if sockets_snapshot.is_empty() {
                rx.changed().await.ok();
                continue;
            }

            let mut recv_stream: FuturesUnordered<_> = sockets_snapshot
                .iter()
                .map(|(key, socket)| {
                    let mut buf = vec![0u8; 1024];
                    let socket = socket.clone();
                    let key = key.clone();
                    async move {
                        match socket.recv_from(&mut buf).await {
                            Ok((len, addr)) => Some((key, socket, addr, buf[..len].to_vec())),
                            Err(_) => None,
                        }
                    }
                })
                .collect();

            select! {
                _ = rx.changed() => continue,

                // Process incoming packets from all sockets
                maybe_packet = recv_stream.next() => {
                    if let Some(Some((_sock, _socket, sender_addr, data))) = maybe_packet {
                        if let Ok(packet) = MeshPacket::decode(&data[..]) {
                            if !self.should_process(&packet.message_id).await { continue; }

                            match packet.packet_type.clone() {
                                Some(PacketType::Heartbeat(hb)) => {
                                    self.update_routing(&hb, sender_addr).await;
                                    self.update_peer_info(&hb.uuid).await;
                                    info!("Received heartbeat from {}: {:?}", hb.uuid, hb);
                                }
                                Some(PacketType::Request(req)) => {
                                    debug!("Received retransmission request: matrix={}, packet={}", req.matrix, req.packet);

                                    // Relay the request if TTL allows
                                    if packet.ttl > 0 {
                                        let mut relay_packet = packet.clone();
                                        relay_packet.ttl -= 1;
                                        let _ = self.relay_packet(&relay_packet).await;
                                    }

                                    self.channels.request_to_mesh.0.send(packet).unwrap_or_else(|e| {
                                        error!("Failed to send request to RetransmissionManager: {}", e);
                                    });


                                }
                                Some(PacketType::Response(resp)) => {
                                    debug!("Received retransmission response: message_id={}", resp.message_id);

                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Process packets from the RetransmissionManager
                maybe_packet = self.channels.request_from_mesh.1.recv_async() => {
                    if let Ok(packet) = maybe_packet {
                        debug!("Received packet from RetransmissionManager: {:?}", packet);

                        if let Err(e) = self.broadcast(&packet).await {
                            error!("Failed to broadcast packet: {}", e);
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
            set.insert(message_id.into());
            true
        }
    }

    // === Message Handling Methods ===

    /// Relay a packet to other nodes in the mesh
    async fn relay_packet(&self, packet: &MeshPacket) -> std::io::Result<()> {
        if packet.ttl <= 0 {
            return Ok(()); // Don't relay packets with expired TTL
        }

        // Broadcast to all connected sockets except the sender
        self.broadcast(packet).await
    }

    /// Send a retransmission request to the mesh network
    pub async fn request_retransmission(
        &self,
        matrix: i32,
        packet_index: i32,
    ) -> std::io::Result<()> {
        let request = RetransmissionRequest {
            matrix,
            packet: packet_index,
        };

        let mesh_packet = MeshPacket {
            message_id: format!("req-{}-{}-{}", self.node_id, matrix, packet_index),
            ttl: 5, // Allow up to 5 hops
            packet_type: Some(PacketType::Request(request)),
        };

        info!(
            "Sending retransmission request for matrix={}, packet={}",
            matrix, packet_index
        );
        self.broadcast(&mesh_packet).await
    }

    /// Send a retransmission response to the mesh network
    pub async fn send_retransmission_response(
        &self,
        message_id: &str,
        payload: Vec<u8>,
    ) -> std::io::Result<()> {
        let response = RetransmissionResponse {
            message_id: message_id.to_string(),
            payload,
        };

        let mesh_packet = MeshPacket {
            message_id: format!("resp-{}-{}", self.node_id, message_id),
            ttl: 5, // Allow up to 5 hops
            packet_type: Some(PacketType::Response(response)),
        };

        info!(
            "Sending retransmission response for message_id={}",
            message_id
        );
        self.broadcast(&mesh_packet).await
    }

    /// Get the next request from the channel (non-blocking)
    pub async fn get_next_request(&self) -> Option<MeshPacket> {
        // This would need access to rx_requests, but it's currently in AppChannels
        // We'll need to modify the architecture slightly
        None
    }

    /// Get the next response from the channel (non-blocking)
    pub async fn get_next_response(&self) -> Option<MeshPacket> {
        // This would need access to rx_responses, but it's currently in AppChannels
        // We'll need to modify the architecture slightly
        None
    }

    /// Get routing information for debugging
    pub async fn get_routing_info(&self) -> HashMap<String, RoutingEntry> {
        self.routing.read().await.clone()
    }

    /// Get peer information
    pub async fn get_peer_info(&self) -> HashMap<String, Instant> {
        self.peer_map.lock().unwrap().clone()
    }

    /// Update peer information when receiving heartbeats
    async fn update_peer_info(&self, uuid: &str) {
        let mut peer_map = self.peer_map.lock().unwrap();
        peer_map.insert(uuid.to_string(), Instant::now());
    }

    /// Clean up old peer entries
    async fn cleanup_peers(peer_map: PeerMap) {
        let mut ticker = interval(Duration::from_secs(15));
        loop {
            ticker.tick().await;
            let mut map = peer_map.lock().unwrap();
            map.retain(|_, last_seen| last_seen.elapsed() < Duration::from_secs(60));
        }
    }
}
