mod network_utils;

#[allow(non_snake_case)]
pub mod android {
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::thread;
    
    use android_logger::{self, init_once};
    use jni::objects::{JClass, JString};
    use jni::JNIEnv;
    use log::{error, info};
    
    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startService(
        _: JNIEnv,
        _: JClass,
        _: JString,
    ) {
        // Init the log of android 
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("RustApp"),
        );

        // Log that the Rust app is starting
        info!("Rust app is starting...");

        use tokio::net::UdpSocket;
        use std::net::{Ipv4Addr, SocketAddrV4};
        use tokio::runtime::Runtime;
        use tokio::io;

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
            let client = iris_client::start(&"stream1.iris.dcc.fc.up.pt".to_string()).unwrap();

    
            for item in client {
                info!("{}",item.len());
            }

        });
    }
}
