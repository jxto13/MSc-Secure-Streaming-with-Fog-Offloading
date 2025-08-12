use log::{error, info, warn};
use std::collections::HashSet;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::IpAddr;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()
        .init();

    let client = Arc::new(RwLock::new(iris_client::Client::new()));

    // let mut node = iris_client::mesh_networking::Node::new();

    // Attempt to start the receiver
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

    // Get safe holder of client
    // if let Ok(client) = client.read() {
    //     let retransmission_manager = client.get_retransmission_manager();

    //     // Get the IPC channels from Node to RetransmissionManager
    //     let ipc_channels = node.get_ipc_channels();

    //     thread::spawn({
    //         let ipc_channels = Arc::clone(&ipc_channels);
    //         move || {
    //             // Correct channel mapping:
    //             // - rx.0 is what retransmission manager uses to send TO mesh (mesh receives via rx.1)
    //             // - tx.1 is what retransmission manager uses to receive FROM mesh (mesh sends via tx.0)
    //             retransmission_manager.middleware(ipc_channels.rx.0.clone(), &ipc_channels.tx.1);
    //         }
    //     });
    // } else {
    //     log::error!("Failed to acquire read lock on client");
    // }

    // // Start a thread to monitor network interfaces
    // thread::spawn(move || {
    //     let prefix = "aware"; // match interfaces starting with this
    //     let mut reported_ifaces: HashSet<String> = HashSet::new();

    //     loop {
    //         let interfaces = pnet_datalink::interfaces();

    //         for iface in &interfaces {
    //             if iface.name.starts_with(prefix) && !reported_ifaces.contains(&iface.name) {
    //                 // Look for IPv6 link-local addresses
    //                 if let Some(link_local) = iface.ips.iter().find_map(|ip_network| {
    //                     if let IpAddr::V6(addr) = ip_network.ip() {
    //                         if addr.is_unicast_link_local() {
    //                             return Some(format!("{}%{}", addr, iface.name));
    //                         }
    //                     }
    //                     None
    //                 }) {
    //                     info!("New interface: {}", iface.name);
    //                     info!("IPv6 link-local: {}", link_local);

    //                     match node.add_socket_sync(&link_local) {
    //                         Ok(()) => {
    //                             info!("Successfully added socket for {}", link_local);
    //                         }
    //                         Err(e) => {
    //                             error!("Failed to add socket for {}: {}", link_local, e);
    //                         }
    //                     }

    //                     // Mark this interface as reported
    //                     reported_ifaces.insert(iface.name.clone());
    //                 }
    //             }
    //         }

    //         thread::sleep(Duration::from_secs(1));
    //     }
    // });

    // Start the ffplay (mpv) process
    let mut ffplay = Command::new("mpv")
        .args(&["-"])
        .stdin(Stdio::piped())
        // .stdout(Stdio::piped()) // Capture stdout for packet handling
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            error!("Failed to start ffplay process: {}", e);
            io::Error::new(io::ErrorKind::Other, "Failed to start ffplay process")
        })?;

    // Get handle to ffplay's stdin
    let mut ffplay_stdin = ffplay.stdin.take().ok_or_else(|| {
        error!("Failed to open ffplay stdin");
        io::Error::new(io::ErrorKind::Other, "Failed to open ffplay stdin")
    })?;

    // Spawn a thread to handle data reception
    let receiver_thread = thread::spawn(move || {
        for data in receiver {
            if let Err(e) = ffplay_stdin.write_all(&data[..]) {
                error!("Failed to write to ffplay stdin: {}", e);
                break;
            }
        }
    });

    // If ffmpeg's stdout is present, read and log it in 188-byte chunks
    if let Some(mut stdout) = ffplay.stdout.take() {
        let mut buffer = [0u8; 188];
        loop {
            match stdout.read_exact(&mut buffer) {
                Ok(_) => {
                    warn!("Received 188-byte packet: {:?}", &buffer);
                }
                Err(e) => {
                    error!("Error reading stdout: {}", e);
                    break;
                }
            }
        }
    }

    // Wait for the receiver thread to finish
    receiver_thread.join().map_err(|e| {
        error!("Receiver thread panicked: {:?}", e);
        io::Error::new(io::ErrorKind::Other, "Receiver thread panicked")
    })?;

    // Wait for the ffplay process to finish
    match ffplay.wait() {
        Ok(status) => info!("ffplay exited with: {}", status),
        Err(e) => {
            error!("Failed to wait on ffplay process: {}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}
