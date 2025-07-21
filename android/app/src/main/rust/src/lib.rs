mod mesh_networking {
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

    // === Protobuf message definition (prost-compatible) ===
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Packet {
        #[prost(string, tag = "1")]
        pub message_id: String,

        #[prost(int32, tag = "2")]
        pub ttl: i32,

        #[prost(bytes, tag = "3")]
        pub payload: Vec<u8>,
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
                let packet = Packet {
                    message_id: format!("heartbeat-{}", self.node_id),
                    ttl: 0,
                    payload: b"heartbeat".to_vec(),
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
                map.retain(|_, &mut last_seen| {
                    now.duration_since(last_seen) < Duration::from_secs(10)
                });
                let active: Vec<String> = map.keys().cloned().collect();
                info!("[{}] Active peers: {:?}", self.node_id, active);
            }
        }

        pub async fn listen_and_relay(self: Arc<Self>) -> std::io::Result<()> {
            let this = self.clone();
            tokio::spawn(this.clone().heartbeat_sender());
            tokio::spawn(this.clone().heartbeat_monitor());
            let mut tasks: HashMap<String, tokio::task::JoinHandle<()>> = HashMap::new();
            let mut rx = self.socket_notify.subscribe();
            loop {
                info!("[{}] Waiting for socket changes...", self.node_id);
                // Check for socket changes
                let sockets = self.sockets.read().await;
                // Start new tasks for new sockets
                for (sock_addr, socket) in sockets.iter() {
                    log::info!("[{}] Detected new socket: {}", self.node_id, sock_addr);
                    if !tasks.contains_key(sock_addr) {
                        let node = self.clone();
                        let sock_addr_task = sock_addr.clone();
                        let sock_addr_map = sock_addr.clone();
                        let socket = socket.clone();
                        let handle = tokio::spawn(async move {
                            let mut buf = vec![0u8; 1024];
                            loop {
                                match socket.recv_from(&mut buf).await {
                                    Ok((len, addr)) => {
                                        let data = &buf[..len];
                                        info!(
                                            "[{}:{}] Received {} bytes from {}",
                                            node.node_id, sock_addr_task, len, addr
                                        );
                                        node.peer_map
                                            .lock()
                                            .unwrap()
                                            .insert(addr.to_string(), Instant::now());
                                        if let Ok(packet) = Packet::decode(data) {
                                            if packet.payload == b"heartbeat" {
                                                let mut peers = node.known_peers.write().await;
                                                if !peers.contains(&addr.to_string())
                                                    && addr.to_string() != sock_addr_task
                                                {
                                                    peers.insert(addr.to_string());
                                                    info!(
                                                        "[{}] Learned new peer: {}",
                                                        node.node_id, addr
                                                    );
                                                }
                                                continue;
                                            }
                                            if packet.ttl <= 0
                                                || !node.should_process(&packet.message_id).await
                                            {
                                                continue;
                                            }
                                            let log_msg = format!(
                                                "[{}:{}] Received from {}: {}",
                                                node.node_id,
                                                sock_addr_task,
                                                addr,
                                                packet.message_id
                                            );
                                            info!("{}", log_msg);

                                            // Parse socket addr string once outside await points
                                            let mut ipv6_addr =
                                                match Self::parse_ipv6_str(&sock_addr_task) {
                                                    Ok(addr) => addr,
                                                    Err(e) => {
                                                        eprintln!(
                                                            "Failed to parse socket addr: {}",
                                                            e
                                                        );
                                                        continue;
                                                    }
                                                };

                                            ipv6_addr.set_port(9060);

                                            if let Err(e) = socket.send_to(b"ack", ipv6_addr).await
                                            {
                                                eprintln!("Failed to send ack: {}", e);
                                            }

                                            let new_packet = Packet {
                                                ttl: packet.ttl - 1,
                                                ..packet
                                            };
                                            let mut encoded = Vec::new();
                                            new_packet.encode(&mut encoded).unwrap();
                                            let peers = node.known_peers.read().await;
                                            for peer in peers.iter() {
                                                if let Ok(target) =
                                                    peer.parse::<std::net::SocketAddr>()
                                                {
                                                    if target != addr {
                                                        socket
                                                            .send_to(&encoded, target)
                                                            .await
                                                            .unwrap();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Socket receive error: {}", e);
                                    }
                                }
                            }
                        });
                        tasks.insert(sock_addr_map, handle);
                    }
                }
                // Stop tasks for removed sockets
                let current_addrs: HashSet<String> = sockets.keys().cloned().collect();
                let to_remove: Vec<String> = tasks
                    .keys()
                    .filter(|k| !current_addrs.contains(*k))
                    .cloned()
                    .collect();
                for addr in to_remove {
                    if let Some(handle) = tasks.remove(&addr) {
                        handle.abort();
                    }
                }
                drop(sockets);
                // Wait for notification of socket change
                let _ = rx.changed().await;
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
}

#[allow(non_snake_case)]
pub mod android {
    use std::io::Write;
    use std::os::fd::{FromRawFd, RawFd};
    use std::sync::{Arc, RwLock};
    use std::thread;

    use jni::objects::{JClass, JString};
    use jni::sys::jint;
    use jni::JNIEnv;
    use log::info;
    use once_cell::sync::OnceCell;
    use std::sync::Once;

    use crate::mesh_networking;

    use mesh_networking::Node;

    // Static variable to hold the iris client instance
    static CLIENT: OnceCell<Arc<RwLock<Option<iris_client::Client>>>> = OnceCell::new();

    // Thread handle to keep track of the spawned thread
    static THREAD_HANDLE: OnceCell<Arc<RwLock<Option<std::thread::JoinHandle<()>>>>> =
        OnceCell::new();

    /// Initializes the logging system for Android.
    static INIT: Once = Once::new();

    static MESH_NETWORK_NODE: OnceCell<Arc<Node>> = OnceCell::new();

    // Initialize the mesh networking module
    pub fn init_mesh_networking() {
        // This function can be used to initialize any mesh networking components if needed
        info!("Mesh networking module initialized");

        let node = mesh_networking::Node::new();
        if MESH_NETWORK_NODE.set(node).is_err() {
            info!("Mesh networking node already initialized");
        } else {
            info!("Mesh networking node created successfully");
        }
    }

    /// Initializes the logging system and panic hook for Android.
    fn init_logging() {
        INIT.call_once(|| {
            // Initialize Android logger
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Trace)
                    .with_tag("IrisClient"),
            );

            // Set up panic hook to log panics to Android logger
            std::panic::set_hook(Box::new(|panic_info| {
                let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                    s
                } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                    s.as_str()
                } else {
                    "Unknown panic payload"
                };

                let location = if let Some(location) = panic_info.location() {
                    format!(" at {}:{}:{}", location.file(), location.line(), location.column())
                } else {
                    String::new()
                };

                log::error!("PANIC: {}{}", payload, location);

                // Try to get thread info
                let binding = std::thread::current();
                let thread_name = binding.name().unwrap_or("unnamed");
                log::error!("Panic occurred in thread: {}", thread_name);

                // Log environment variable hint for backtrace
                log::error!("To enable backtrace, set RUST_BACKTRACE=1 environment variable before starting the application");
            }));

            log::info!("IrisClient logging and panic handling initialized for Android");            
        });
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startService(
        _env: JNIEnv,
        _class: JClass,
    ) {
        // Initialize the mesh networking node and logging
        init_logging();
        init_mesh_networking();
    }

    /// Starts the mesh networking node and its tasks.
    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startClient(
        mut env: JNIEnv,
        _class: JClass,
        fd: jint,
        stream_url: JString,
    ) -> jni::sys::jboolean {
        init_logging();

        info!("Rust client is starting...");

        let fd = fd as RawFd;

        // Convert JString to Rust String
        let url_str: String = match env.get_string(&stream_url) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert stream URL from Java: {}", e);
                return jni::sys::JNI_FALSE;
            }
        };

        info!("Starting client with URL: {}", url_str);

        // Try to start the client and send result
        let mut client = iris_client::Client::new();

        match client.start(&url_str) {
            Ok(data_receiver) => {
                info!("Successfully connected to server");

                // Initialize CLIENT static if not already done
                if CLIENT.get().is_none() {
                    let _ = CLIENT.set(Arc::new(RwLock::new(None)));
                }
                if THREAD_HANDLE.get().is_none() {
                    let _ = THREAD_HANDLE.set(Arc::new(RwLock::new(None)));
                }

                // Store the client
                if let Some(client_holder) = CLIENT.get() {
                    if let Ok(mut guard) = client_holder.write() {
                        *guard = Some(client);
                    }
                }

                let handle = thread::spawn(move || {
                    // SAFETY: fd must be valid and owned by Java
                    let file = unsafe { std::fs::File::from_raw_fd(fd) };

                    // Clone so Rust can write, but Java still owns fd
                    let mut writer = match file.try_clone() {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Failed to clone fd: {:?}", e);
                            return;
                        }
                    };
                    // Don't close the original fd owned by Java
                    std::mem::forget(file); // avoid closing fd

                    // Loop over incoming data
                    for item in data_receiver {
                        if let Err(e) = writer.write_all(&item) {
                            eprintln!("Write error: {:?}", e);
                            break;
                        }
                    }
                    log::error!("Client stream ended for URL: {}", url_str);
                });

                // Store the thread handle
                if let Some(handle_holder) = THREAD_HANDLE.get() {
                    if let Ok(mut guard) = handle_holder.write() {
                        *guard = Some(handle);
                    }
                }

                jni::sys::JNI_TRUE
            }
            Err(e) => {
                log::error!("Failed to start iris client with URL {}: {:?}", url_str, e);
                jni::sys::JNI_FALSE
            }
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_stopClient(
        _env: JNIEnv,
        _class: JClass,
    ) {
        log::info!("Stopping iris client...");

        // Shutdown the client first
        if let Some(client_holder) = CLIENT.get() {
            if let Ok(mut guard) = client_holder.write() {
                if let Some(client) = guard.take() {
                    client.shutdown();
                    log::info!("Iris client shutdown requested");
                }
            }
        }

        // Wait for the thread to finish
        if let Some(handle_holder) = THREAD_HANDLE.get() {
            if let Ok(mut guard) = handle_holder.write() {
                if let Some(handle) = guard.take() {
                    match handle.join() {
                        Ok(_) => log::info!("Thread joined successfully"),
                        Err(e) => log::error!("Error joining thread: {:?}", e),
                    }
                }
            }
        }

        log::info!("Iris client shutdown complete");
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_addSocket(
        mut env: JNIEnv,
        _class: JClass,
        ip_address: JString,
    ) {
        // Convert JString to Rust String
        let ip_str: String = match env.get_string(&ip_address) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert IP address from Java: {}", e);
                return;
            }
        };

        log::info!("Adding socket for IP: {}", ip_str);

        // Get node instance
        let node = match MESH_NETWORK_NODE.get() {
            Some(node) => node.clone(),
            None => {
                log::error!("Mesh networking node is not initialized");
                return;
            }
        };

        // Add socket to the node
        match node.add_socket_sync(&ip_str) {
            Ok(_) => log::info!("Successfully added socket for IP: {}", ip_str),
            Err(e) => log::error!("Failed to add socket for IP {}: {:?}", ip_str, e),
        };

        // SAFETY: CLIENT is a static variable, so we can safely access it
        // if let Some(client_holder) = CLIENT.get() {
        //     if let Ok(guard) = client_holder.read() {
        //         if let Some(client) = guard.as_ref() {
        //             match client.add_interface(&ip_str) {
        //                 Ok(_) => log::info!("Successfully added socket for IP: {}", ip_str),
        //                 Err(e) => log::error!("Failed to add socket for IP {}: {:?}", ip_str, e),
        //             }
        //         } else {
        //             log::error!("Iris client is not initialized");
        //         }
        //     } else {
        //         log::error!("Failed to acquire read lock on client");
        //     }
        // } else {
        //     log::error!("Iris client holder is not initialized");
        // }
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_getPacketLossPercentage(
        _env: JNIEnv,
        _class: JClass,
    ) -> f64 {
        // TODO: Implement actual packet loss calculation from iris_client
        // For now, return a mock value that changes over time
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let variation = (now % 20) as f64 * 0.1; // 0.0 to 2.0% variation
        let result = 0.5 + variation; // Base 0.5% + variation
        log::trace!("Returning packet loss percentage: {:.2}%", result);
        result
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_getCurrentBitrateMbps(
        _env: JNIEnv,
        _class: JClass,
    ) -> f64 {
        // TODO: Implement actual bitrate calculation from iris_client
        // For now, return a mock value that simulates realistic bitrate
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let variation = (now % 30) as f64 * 0.5; // 0.0 to 15.0 Mbps variation
        let result = 25.0 + variation; // Base 25 Mbps + variation
        log::trace!("Returning current bitrate: {:.2} Mbps", result);
        result
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_getTotalDataTransferredMB(
        _env: JNIEnv,
        _class: JClass,
    ) -> i64 {
        // TODO: Implement actual data transfer tracking from iris_client
        // For now, return a mock value that increases over time
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let result = (now / 10) as i64; // Increases by 1 MB every 10 seconds
        log::trace!("Returning total data transferred: {} MB", result);
        result
    }
}
