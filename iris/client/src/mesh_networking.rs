use log::{debug, error, info};
use pnet_datalink;
use prost::Message;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;
use tokio::sync::{watch, RwLock};
use tokio::time::interval;
use uuid::Uuid;

use crate::proto::mesh_packet::PacketType;
use crate::proto::{HeartbeatPacket, MeshPacket};

// === Shared Types ===
pub type PeerMap = Arc<Mutex<HashMap<String, Instant>>>;

// Channels for application-level request/response injection
pub struct NodeChannels {
    // tx channel: Mesh -> RetransmissionManager
    // tx.0 = Sender (mesh uses this to send TO retransmission manager)
    // tx.1 = Receiver (retransmission manager uses this to receive FROM mesh)
    pub tx: (loole::Sender<MeshPacket>, loole::Receiver<MeshPacket>),

    // rx channel: RetransmissionManager -> Mesh
    // rx.0 = Sender (retransmission manager uses this to send TO mesh)
    // rx.1 = Receiver (mesh uses this to receive FROM retransmission manager)
    pub rx: (loole::Sender<MeshPacket>, loole::Receiver<MeshPacket>),
}

pub struct Node {
    pub node_id: String,
    pub peer_map: PeerMap,
    pub sockets: Arc<RwLock<HashMap<String, Arc<UdpSocket>>>>,
    pub socket_notify: watch::Sender<()>,
    pub channels: Arc<NodeChannels>,
    pub runtime: Arc<Runtime>,
}

impl Node {
    pub fn new() -> Arc<Self> {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        let (tx_notify, _) = watch::channel(());

        let node = Arc::new(Self {
            node_id: Uuid::new_v4().to_string(),
            peer_map: Arc::new(Mutex::new(HashMap::new())),
            sockets: Arc::new(RwLock::new(HashMap::new())),
            socket_notify: tx_notify,
            channels: Arc::new(NodeChannels {
                tx: loole::unbounded(),
                rx: loole::unbounded(),
            }),
            runtime: Arc::new(runtime),
        });

        info!("Node {} initialized", node.node_id);

        // Launch tasks
        let node_clone = node.clone();
        node.runtime.spawn(async move {
            if let Err(e) = node_clone.listen_and_relay().await {
                error!("listen_and_relay exited with error: {}", e);
            }
        });

        // Peer cleanup
        let peer_map = node.peer_map.clone();
        node.runtime.spawn(async move {
            Node::cleanup_peers(peer_map).await;
        });

        node
    }

    pub fn get_ipc_channels(&self) -> Arc<NodeChannels> {
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

        // Optimize socket buffer sizes for better throughput
        // socket.set_recv_buffer_size(131072)?; // 128KB
        // socket.set_send_buffer_size(131072)?; // 128KB

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

    // === Heartbeat ===
    async fn heartbeat_sender(self: Arc<Self>) {
        use tokio::select;
        let mut ticker = interval(Duration::from_secs(5));
        let mut rx = self.socket_notify.subscribe();

        loop {
            select! {
                _ = ticker.tick() => {
                    // Get peer list from peer map instead of routing
                    let peer_list: Vec<String> = {
                        let peer_map = self.peer_map.lock().unwrap();
                        peer_map.keys().cloned().collect()
                    };

                    let hb = MeshPacket {
                        ttl: 3,
                        packet_type: Some(PacketType::Heartbeat(HeartbeatPacket {
                            uuid: self.node_id.clone(),
                            addr: "".into(),
                            peers: peer_list,
                        })),
                    };

                    let mut buf = Vec::with_capacity(512);
                    hb.encode(&mut buf).unwrap();

                    let sockets = self.sockets.read().await;

                    // Send heartbeats concurrently to all sockets
                    let send_futures: Vec<_> = sockets
                        .iter()
                        .filter_map(|(sock_addr, socket)| {
                            if let Ok(mut ipv6) = Self::parse_ipv6_str(sock_addr) {
                                ipv6.set_port(9060);
                                let buf = buf.clone();
                                let socket = socket.clone();
                                let addr_str = sock_addr.clone();
                                Some(async move {
                                    if let Err(e) = socket.send_to(&buf, ipv6).await {
                                        debug!("Failed to send heartbeat to {}: {}", addr_str, e);
                                    }
                                })
                            } else {
                                debug!("Invalid IPv6 address for socket: {}", sock_addr);
                                None
                            }
                        })
                        .collect();

                    // Execute all sends concurrently
                    futures::future::join_all(send_futures).await;
                }
                _ = rx.changed() => {
                    debug!("Socket list changed, heartbeat sender will use updated sockets.");
                }
            }
        }
    }

    async fn process_heartbeat(&self, hb: &HeartbeatPacket) {
        // Simply update peer information
        self.update_peer_info(&hb.uuid).await;
        debug!("Updated peer info for {}", hb.uuid);
    }

    async fn broadcast(&self, packet: &MeshPacket) -> std::io::Result<()> {
        let mut buf = Vec::with_capacity(1024); // Pre-allocate capacity
        packet.encode(&mut buf).unwrap();

        let sockets = self.sockets.read().await;

        // Use parallel sending for better performance
        let send_futures: Vec<_> = sockets
            .iter()
            .filter_map(|(sock_addr, socket)| {
                if let Ok(mut ipv6) = Self::parse_ipv6_str(sock_addr) {
                    ipv6.set_port(9060);
                    let buf = buf.clone();
                    let socket = socket.clone();
                    Some(async move { socket.send_to(&buf, ipv6).await })
                } else {
                    None
                }
            })
            .collect();

        // Send to all sockets concurrently
        let results = futures::future::join_all(send_futures).await;

        // Check if any sends failed and return the first error
        for result in results {
            if let Err(e) = result {
                return Err(e);
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

        // Increased buffer size for better performance
        const BUFFER_SIZE: usize = 2048;

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

            // Create multiple concurrent receive tasks for each socket for better throughput
            let mut recv_stream: FuturesUnordered<_> = sockets_snapshot
                .iter()
                .flat_map(|(key, socket)| {
                    // Create multiple concurrent receivers per socket
                    (0..3).map(move |i| {
                        let mut buf = vec![0u8; BUFFER_SIZE];
                        let socket = socket.clone();
                        let key = format!("{}-{}", key, i);
                        async move {
                            match socket.recv_from(&mut buf).await {
                                Ok((len, addr)) => Some((key, socket, addr, buf[..len].to_vec())),
                                Err(_) => None,
                            }
                        }
                    })
                })
                .collect();

            select! {
                _ = rx.changed() => continue,

                // Process incoming packets from all sockets with async processing
                mesh_incoming_packet = recv_stream.next() => {
                    if let Some(Some((_sock, _socket, sender_addr, data))) = mesh_incoming_packet {
                        // Process packet in a separate task to avoid blocking the main loop
                        let self_clone = self.clone();
                        tokio::spawn(async move {
                            if let Ok(packet) = MeshPacket::decode(&data[..]) {
                                // Process all packets without duplicate checking

                                match packet.packet_type.clone() {
                                    Some(PacketType::Heartbeat(hb)) => {
                                        // Process heartbeat
                                        self_clone.process_heartbeat(&hb).await;
                                        debug!("Received heartbeat from {}: {:?}", hb.uuid, hb);

                                        // Relay heartbeat if TTL allows and it's not from ourselves
                                        if packet.ttl > 0 && hb.uuid != self_clone.node_id {
                                            let mut relay_packet = packet.clone();
                                            relay_packet.ttl -= 1;
                                            let self_for_relay = self_clone.clone();

                                            tokio::spawn(async move {
                                                if let Err(e) = self_for_relay.broadcast_with_loop_prevention(&relay_packet, Some(sender_addr)).await {
                                                    debug!("Failed to relay heartbeat: {}", e);
                                                }
                                            });
                                        }
                                    }
                                    Some(PacketType::Request(_)) => {
                                        debug!("Received retransmission request");

                                        // Relay the request if TTL allows
                                        if packet.ttl > 0 {
                                            let mut relay_packet = packet.clone();
                                            relay_packet.ttl -= 1;
                                            let self_for_relay = self_clone.clone();

                                            tokio::spawn(async move {
                                                if let Err(e) = self_for_relay.broadcast_with_loop_prevention(&relay_packet, Some(sender_addr)).await {
                                                    debug!("Failed to relay request: {}", e);
                                                }
                                            });
                                        }

                                        // Send to RetransmissionManager
                                        if let Err(e) = self_clone.channels.tx.0.send(packet) {
                                            error!("Failed to send request to RetransmissionManager: {}", e);
                                        } else {
                                            debug!("Successfully sent packet to RetransmissionManager");
                                        }
                                    }
                                    Some(PacketType::Response(_)) => {
                                        // debug!("Received retransmission response");

                                        // Relay response if TTL allows
                                        if packet.ttl > 0 {
                                            let mut relay_packet = packet.clone();
                                            relay_packet.ttl -= 1;
                                            let self_for_relay = self_clone.clone();

                                            tokio::spawn(async move {
                                                if let Err(e) = self_for_relay.broadcast_with_loop_prevention(&relay_packet, Some(sender_addr)).await {
                                                    debug!("Failed to relay response: {}", e);
                                                }
                                            });
                                        }

                                        // Send to RetransmissionManager
                                        if let Err(e) = self_clone.channels.tx.0.send(packet) {
                                            error!("Failed to send response to RetransmissionManager: {}", e);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        });
                    }
                }

                // Process packets from the RetransmissionManager with high priority
                manager_packet = self.channels.rx.1.recv_async() => {
                    if let Ok(packet) = manager_packet {
                        // debug!("Mesh received packet from RetransmissionManager");

                        // Broadcast in separate task to avoid blocking
                        let self_clone = self.clone();
                        tokio::spawn(async move {
                            if let Err(e) = self_clone.broadcast(&packet).await {
                                error!("Failed to broadcast packet: {}", e);
                            }
                        });
                    }
                }
            }
        }
    }

    // === Message Handling Methods ===

    /// Broadcast with loop prevention
    async fn broadcast_with_loop_prevention(
        &self,
        packet: &MeshPacket,
        exclude_sender: Option<SocketAddr>,
    ) -> std::io::Result<()> {
        let mut buf = Vec::with_capacity(1024);
        packet.encode(&mut buf).unwrap();

        let sockets = self.sockets.read().await;

        let send_futures: Vec<_> = sockets
            .iter()
            .filter_map(|(sock_addr, socket)| {
                if let Ok(mut ipv6) = Self::parse_ipv6_str(sock_addr) {
                    ipv6.set_port(9060);

                    // Skip sending back to the sender to prevent immediate loops
                    if let Some(sender) = exclude_sender {
                        if let std::net::IpAddr::V6(sender_ipv6) = sender.ip() {
                            if ipv6.ip() == &sender_ipv6 {
                                return None;
                            }
                        }
                    }

                    let buf = buf.clone();
                    let socket = socket.clone();
                    Some(async move { socket.send_to(&buf, ipv6).await })
                } else {
                    None
                }
            })
            .collect();

        let results = futures::future::join_all(send_futures).await;

        for result in results {
            if let Err(e) = result {
                return Err(e);
            }
        }

        Ok(())
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
        let mut ticker = interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            let mut map = peer_map.lock().unwrap();
            map.retain(|_, last_seen| last_seen.elapsed() < Duration::from_secs(30));
        }
    }
}
