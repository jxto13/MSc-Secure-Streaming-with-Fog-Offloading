use log::{debug, error, info};
use pnet_datalink;
use prost::Message;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, Ipv6Addr, SocketAddr, SocketAddrV6};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;
use tokio::sync::{watch, RwLock};
use tokio::time::{interval, Interval};
use uuid::Uuid;

use crate::proto::mesh_packet::PacketType;
use crate::proto::{HeartbeatPacket, MeshPacket};

// === Shared Types ===
pub type PeerMap = Arc<Mutex<HashMap<String, Instant>>>;

// Channels for application-level request/response injection
pub struct NodeChannels {
    pub tx: (loole::Sender<MeshPacket>, loole::Receiver<MeshPacket>),
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

        // Heartbeats
        let node_clone = node.clone();
        node.runtime.spawn(async move {
            if let Err(e) = node_clone.heartbeat_sender().await {
                error!("heartbeat_sender error: {}", e);
            }
        });

        // Dispatcher
        let node_clone = node.clone();
        node.runtime.spawn(async move {
            if let Err(e) = node_clone.dispatch().await {
                error!("dispatcher exited: {}", e);
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

    // === Interface helpers (IPv6 only) ===

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
                    if let IpAddr::V6(ipv6) = ip_net.ip() {
                        return Ok((ipv6, iface.index));
                    }
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Interface '{}' not found or has no IPv6", name),
        ))
    }

    // === Address parsing (IPv6 only) ===

    fn parse_bind_target(s: &str) -> std::io::Result<SocketAddr> {
        // Accept "fe80::...%ifname" or a bare interface name.
        if let Some((host, ifname)) = s.split_once('%') {
            let v6 = Ipv6Addr::from_str(host)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
            if !v6.is_unicast_link_local() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "scope only valid for IPv6 link-local",
                ));
            }
            let scope_id = Self::get_interface_index(ifname)?;
            return Ok(SocketAddr::V6(SocketAddrV6::new(v6, 9060, 0, scope_id)));
        }

        let (v6, scope) = Self::get_local_interface_from_client_ipv6(s)?;
        Ok(SocketAddr::V6(SocketAddrV6::new(v6, 9060, 0, scope)))
    }

    fn key_to_sockaddr(key: &str) -> Option<SocketAddr> {
        Self::parse_bind_target(key).ok()
    }

    // === Socket creation (IPv6 only) ===

    async fn create_reusable_udp_socket(addr: SocketAddrV6) -> std::io::Result<UdpSocket> {
        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        #[cfg(unix)]
        socket.set_reuse_port(true).ok(); // best effort

        socket.set_only_v6(true)?;
        socket
            .bind(&addr.into())
            .map_err(|e| io::Error::new(e.kind(), format!("bind {} failed: {e}", addr)))?;

        let std_socket: std::net::UdpSocket = socket.into();
        std_socket.set_nonblocking(true)?;
        Ok(UdpSocket::from_std(std_socket)?)
    }

    // === Public API ===

    pub fn add_socket_sync(&self, addr: &str) -> std::io::Result<()> {
        self.runtime.block_on(self.add_socket(addr))
    }

    pub async fn add_socket(&self, addr: &str) -> std::io::Result<()> {
        // Expect "fe80::...%ifname"; fall back to just interface after '%'
        let ifname = addr.split('%').nth(1).unwrap_or(addr);
        let (ipv6, scope_id) = Self::get_local_interface_from_client_ipv6(ifname)?;
        let bind = SocketAddrV6::new(ipv6, 9060, 0, scope_id);

        let socket = Arc::new(Self::create_reusable_udp_socket(bind).await?);

        {
            let mut map = self.sockets.write().await;
            map.insert(addr.to_string(), socket);
        }

        let _ = self.socket_notify.send(());
        info!("Added IPv6 socket on iface {ifname} (scope {scope_id})");
        Ok(())
    }

    pub fn remove_socket_sync(&self, addr: &str) {
        self.runtime.block_on(self.remove_socket(addr));
    }

    pub async fn remove_socket(&self, addr: &str) {
        self.sockets.write().await.remove(addr);
        let _ = self.socket_notify.send(());
        info!("Removed socket {}", addr);
    }

    // === Heartbeat ===

    async fn heartbeat_sender(self: Arc<Self>) -> std::io::Result<()> {
        use tokio::select;
        let mut ticker = interval(Duration::from_secs(5));
        let mut rx = self.socket_notify.subscribe();

        loop {
            select! {
                _ = ticker.tick() => {
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
                    for (sock_key, socket) in sockets.iter() {
                        if let Some(SocketAddr::V6(mut sa)) = Self::key_to_sockaddr(sock_key) {
                            sa.set_port(9060);
                            if let Err(e) = socket.send_to(&buf, sa).await {
                                debug!("Failed to send heartbeat to {}: {}", sock_key, e);
                            }
                        }
                    }
                }
                _ = rx.changed() => {
                    debug!("Socket list changed; heartbeat using updated sockets");
                }
            }
        }
    }

    // === Dispatcher: polling all sockets for packets ===

    async fn dispatch(self: Arc<Self>) -> std::io::Result<()> {
        use tokio::select;

        // Polling interval keeps CPU modest while sockets are nonblocking.
        let mut poll: Interval = interval(Duration::from_millis(2));
        let mut rx_notify = self.socket_notify.subscribe();

        loop {
            select! {
                _ = poll.tick() => {
                    // Drain all available packets from all sockets.
                    let sockets = self.sockets.read().await;
                    for (key, socket) in sockets.iter() {
                        loop {
                            let mut buf = [0u8; 2048];
                            match socket.try_recv_from(&mut buf) {
                                Ok((n, from)) => {
                                    let data = &buf[..n];
                                    if let Ok(packet) = MeshPacket::decode(data) {
                                        match packet.packet_type.clone() {
                                            Some(PacketType::Heartbeat(hb)) => {
                                                self.update_peer_info(&hb.uuid).await;
                                                debug!("Received heartbeat on {} from {}: {:?}", key, from, hb);

                                                if packet.ttl > 0 && hb.uuid != self.node_id {
                                                    let mut relay = packet.clone();
                                                    relay.ttl -= 1;
                                                    if let Err(e) = self.broadcast(&relay, Some(from)).await {
                                                        debug!("Failed to relay heartbeat: {}", e);
                                                    }
                                                }
                                            }
                                            Some(PacketType::Request(_)) => {
                                                debug!("Received retransmission request on {} from {}", key, from);
                                                if packet.ttl > 0 {
                                                    let mut relay = packet.clone();
                                                    relay.ttl -= 1;
                                                    if let Err(e) = self.broadcast(&relay, Some(from)).await {
                                                        debug!("Failed to relay request: {}", e);
                                                    }
                                                }
                                                if let Err(e) = self.channels.tx.0.send(packet) {
                                                    error!("Failed to send request to RetransmissionManager: {}", e);
                                                }
                                            }
                                            Some(PacketType::Response(_)) => {
                                                if packet.ttl > 0 {
                                                    let mut relay = packet.clone();
                                                    relay.ttl -= 1;
                                                    if let Err(e) = self.broadcast(&relay, Some(from)).await {
                                                        debug!("Failed to relay response: {}", e);
                                                    }
                                                }
                                                if let Err(e) = self.channels.tx.0.send(packet) {
                                                    error!("Failed to send response to RetransmissionManager: {}", e);
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    break; // no more data on this socket now
                                }
                                Err(e) => {
                                    debug!("recv error on {}: {}", key, e);
                                    break; // avoid tight loop on persistent error
                                }
                            }
                        }
                    }
                }

                // Packets coming from the manager to broadcast
                manager_packet = self.channels.rx.1.recv_async() => {
                    if let Ok(packet) = manager_packet {
                        if let Err(e) = self.broadcast(&packet, None).await {
                            error!("Failed to broadcast packet: {}", e);
                        }
                    }
                }

                // React to socket set changes promptly
                _ = rx_notify.changed() => {
                    debug!("Dispatcher noticed socket set change");
                }
            }
        }
    }

    // === Message Handling (merged) ===

    async fn broadcast(
        &self,
        packet: &MeshPacket,
        exclude_sender: Option<SocketAddr>,
    ) -> std::io::Result<()> {
        let mut buf = Vec::with_capacity(1024);
        packet.encode(&mut buf).unwrap();

        let sockets = self.sockets.read().await;
        for (sock_key, socket) in sockets.iter() {
            if let Some(SocketAddr::V6(mut sa)) = Self::key_to_sockaddr(sock_key) {
                sa.set_port(9060);

                if let Some(SocketAddr::V6(sender_v6)) = exclude_sender {
                    if *sa.ip() == *sender_v6.ip() {
                        continue;
                    }
                }

                debug!("Forwarding packet {:?} to {}", packet.packet_type, sa);
                socket.send_to(&buf, sa).await?;
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
