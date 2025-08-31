use clap::{arg, Command};
use log::{error, info};
use std::collections::HashSet;
use std::error::Error;
use std::io::{self, Read};
use std::net::IpAddr;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn Error>> {
    // tracing_subscriber::fmt()
    //     .with_max_level(tracing::Level::TRACE)
    //     .init();

    // Init the logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()
        .init();

    let cmd = Command::new("streaming_server")
        .version("0.1.0")
        .author("Joaquim Oliveira <up201908075@up.pt>")
        .about("MSc Secure Streaming with Fog Offloading")
        .arg(
            arg!(--"ip" <IP_ADDRESS>)
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to the configuration file"),
        );

    let matches = cmd.get_matches();
    let ip_path = matches.get_one::<PathBuf>("ip").expect("required argument");

    let ip_str: &str = ip_path.to_str().expect("Invalid UTF-8 in path");
    let ip_string: String = ip_str.to_string();

    // Channel for ESC signal
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    // Spawn thread to listen for ESC key
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        let mut buffer = [0; 1];

        loop {
            if let Ok(_) = handle.read_exact(&mut buffer) {
                if buffer[0] == 27 {
                    // ESC key
                    info!("ESC pressed. Sending stop signal.");
                    let _ = stop_tx.send(()); // Ignore send error
                    break;
                }
            }
        }
    });

    // Start client
    let client = Arc::new(RwLock::new(iris_client::Client::new()));

    // Create a new mesh networking node
    let mut node = iris_client::mesh_networking::Node::new();

    // Start the client
    let receiver = match client
        .write()
        .unwrap()
        .start(&"stream1.iris.dcc.fc.up.pt".to_string())
    {
        Ok(receiver) => receiver,
        Err(e) => {
            error!("Failed to start receiver: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to start receiver").into());
        }
    };

    // Start the retransmission manager

    // Get safe holder of client
    if let Ok(client) = client.read() {
        let retransmission_manager = client.get_retransmission_manager();

        // Get the IPC channels from Node to RetransmissionManager
        let ipc_channels = node.get_ipc_channels();

        thread::spawn({
            let ipc_channels = Arc::clone(&ipc_channels);
            move || {
                // Correct channel mapping:
                // - rx.0 is what retransmission manager uses to send TO mesh (mesh receives via rx.1)
                // - tx.1 is what retransmission manager uses to receive FROM mesh (mesh sends via tx.0)
                retransmission_manager.middleware(ipc_channels.rx.0.clone(), &ipc_channels.tx.1);
            }
        });
    } else {
        log::error!("Failed to acquire read lock on client");
    }

    // Listen to interfaces (simulates client p2p)
    thread::spawn(move || {
        let mut seen: HashSet<String> = HashSet::new();

        loop {
            let interfaces = pnet_datalink::interfaces();
            let mut current: HashSet<String> = HashSet::new();

            for iface in interfaces.iter().filter(|i| i.name != "eth0") {
                // consider all IPs on the iface; keep as-is if you only want the first
                for ip_network in &iface.ips {
                    if let IpAddr::V6(addr) = ip_network.ip() {
                        if addr.is_unicast_link_local() {
                            let link_local = format!("{}%{}", addr, iface.name);
                            current.insert(link_local.clone());

                            if !seen.contains(&link_local) {
                                info!("New interface: {}", iface.name);
                                info!("IPv6 link-local: {}", link_local);

                                match node.add_socket_sync(&link_local) {
                                    Ok(()) => info!("Successfully added socket for {}", link_local),
                                    Err(e) => {
                                        error!("Failed to add socket for {}: {}", link_local, e)
                                    }
                                }

                                seen.insert(link_local);
                            }
                        }
                    }
                }
            }

            // Optional: drop sockets for vanished addresses
            // seen.retain(|k| current.contains(k));

            thread::sleep(Duration::from_secs(1));
        }
    });

    for _item in receiver {
        if stop_rx.try_recv().is_ok() {
            info!("Stop signal received. Shutting down receiver...");
            client.read().unwrap().shutdown();
            break;
        }

        // Process item if needed
    }

    Ok(())
}
