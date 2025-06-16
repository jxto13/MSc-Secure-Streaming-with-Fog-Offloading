mod network_utils;

#[allow(non_snake_case)]
pub mod android {
    use std::io::Write;
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::os::fd::{FromRawFd, RawFd};
    use std::thread;
    
    use android_logger::{self, init_once};
    use jni::objects::{JByteBuffer, JClass, JString};
    use jni::sys::jint;
    use jni::JNIEnv;
    use log::{error, info};
    use once_cell::sync::Lazy;
    use tokio::runtime::Runtime;
    use std::time::{Duration, Instant};
    
    static TOKIO_RT: Lazy<Runtime> = Lazy::new(|| {
        Runtime::new().expect("Failed to create Tokio runtime")
    });

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startClient(
        _env: JNIEnv,
        _class: JClass,
        fd: jint,
    ) {
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Trace)
                .with_tag("RustApp"),
        );

        info!("Rust client is starting...");

        let fd = fd as RawFd;

        thread::spawn(move || {
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

            // Start the iris client
            let client = match iris_client::start_client(&"stream1.iris.dcc.fc.up.pt".to_string()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to start iris client: {:?}", e);
                    return;
                }
            };

            // Loop over incoming data
            for item in client {

                if let Err(e) = writer.write_all(&item) {
                    eprintln!("Write error: {:?}", e);
                    break;
                }
            }

            eprintln!("Client stream ended.");
        });
        

        // let message = b"Hello from Rust via ParcelFileDescriptor!\n";

        // unsafe {
        //     // Convert raw fd into Rust File
        //     let file = std::fs::File::from_raw_fd(fd);

        //     // Clone to avoid closing the original fd (Java still owns it)
        //     let mut cloned = file.try_clone().expect("Failed to clone fd");

        //     // Write to it
        //     if let Err(e) = cloned.write_all(message) {
        //         eprintln!("Failed to write: {:?}", e);
        //     }

        //     // Don't close the original fd owned by Java
        //     std::mem::forget(file);
        // }

        // Spawn Tokio task on the runtime
        // thread::spawn(move || {
        //     TOKIO_RT.spawn(async move {
        //         info!("Running inside Tokio runtime.");
        //         // receive_udp_packets(buffer_ptr.as_mut_ptr(), buf_len).await;

        //         let socket = UdpSocket::bind("192.168.1.131:1234")
        //             .await
        //             .expect("Failed to bind UDP socket");

        //         println!("UDP listener started on 192.168.1.131:1234");


        //         // SAFELY wrap fd
        //         let file = unsafe { std::fs::File::from_raw_fd(fd) };
        //         let mut writer = match file.try_clone() {
        //             Ok(f) => f,
        //             Err(e) => {
        //                 eprintln!("Failed to clone fd: {:?}", e);
        //                 return;
        //             }
        //         };

        //         // Don't close Java's fd
        //         std::mem::forget(file);

        //         let mut buf = [0u8; 4096];

        //         loop {
        //             match socket.recv_from(&mut buf).await {
        //                 Ok((size, addr)) => {
        //                     println!("Received {} bytes from {}", size, addr);

        //                     if let Err(e) = writer.write_all(&buf[..size]) {
        //                         eprintln!("Failed to write to pipe: {:?}", e);
        //                         break;
        //                     }
        //                 }
        //                 Err(e) => {
        //                     eprintln!("UDP receive error: {:?}", e);
        //                     break;
        //                 }
        //             }
        //         }
        //     });
        // });
    }


    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startService(
        _: JNIEnv,
        _: JClass,
        _: JString,
    ) {
        // Init the log of android 
        init_once(
    android_logger::Config::default()
                .with_max_level(log::LevelFilter::Trace)
                .with_tag("RustApp"),
        );

        // Log that the Rust app is starting
        info!("Rust app is starting...");

        use tokio::net::UdpSocket;
        use std::net::{Ipv4Addr, SocketAddrV4};
        use tokio::runtime::Runtime;
        use tokio::io;

        // Spawn async task without blocking the JNI thread
        // thread::spawn(|| {
        //     TOKIO_RT.spawn(async {
        //         info!("Running inside Tokio runtime.");
        //         // Insert your async logic here
        //         receive_multicast_packets().await;
        //     });
        // });

        // thread::spawn(||{

        //     // Create a new Tokio runtime
        //     let rt = Runtime::new().unwrap();
            
        //     rt.block_on(async {
        //         let multicast_addr: Ipv4Addr = "239.255.0.1".parse().expect("Invalid multicast address");
        //         let port = 9999;
        //         let local_addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();

        //         let socket = UdpSocket::bind(local_addr).await.expect("Failed to bind socket");
                
        //         // Join the multicast group
        //         socket.join_multicast_v4(multicast_addr, Ipv4Addr::UNSPECIFIED).expect("Failed to join multicast group");
                
        //         const MAX_DATAGRAM_SIZE: usize = 65_507;
        //         let mut data = vec![0u8; MAX_DATAGRAM_SIZE];

        //         println!("Listening for multicast UDP packets on {}:{}", multicast_addr, port);

        //         loop {
        //             let (len, addr) = socket.recv_from(&mut data).await.expect("Failed to receive data");
        //             info!("Received {} bytes from {}", len, addr);
        //         }
        //     })
        // });

        
        // fn is_same_subnet(ip1: Ipv4Addr, ip2: Ipv4Addr, netmask: Ipv4Addr) -> bool {
        //     let ip1_int = u32::from(ip1) & u32::from(netmask);
        //     let ip2_int = u32::from(ip2) & u32::from(netmask);
        //     ip1_int == ip2_int
        // }
    

        // use pnet::datalink;
        // use std::net::{IpAddr,Ipv4Addr};

        // // Domain name to resolve
        // let domain = "stream1.iris.dcc.fc.up.pt";

        // // Use lookup_host to resolve the domain name
        // let addrs = (domain, 80).to_socket_addrs().unwrap();

        // for addr in addrs {
        //     if let SocketAddr::V4(v4) = addr {
        //         let resolved_ip = v4.ip().clone();
        //         info!("Resolved IP: {}", resolved_ip);
    
        //         for interface in datalink::interfaces() {
        //             for ip_net in &interface.ips {
        //                 if let IpAddr::V4(ip) = ip_net.ip() {
        //                     if let IpAddr::V4(netmask) = ip_net.mask() {
        //                         if is_same_subnet(ip, resolved_ip, netmask) {
        //                             info!("Interface {} has IPv4 {} on the same subnet as {}", 
        //                                  interface.name, ip, resolved_ip);
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        // }
    

        thread::spawn(||{
           let client = iris_client::start_client(&"stream1.iris.dcc.fc.up.pt".to_string()).unwrap();

    
           for item in client {
           }

        });
    }
}
