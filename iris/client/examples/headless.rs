use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;
use log::{error, info};
use std::io;
use clap::{arg, Command};
use std::net::ToSocketAddrs;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()
        .init();

    // Define the command structure
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

    // Parse command-line arguments
    let matches = cmd.get_matches();
    let ip_path = matches.get_one::<PathBuf>("ip").expect("required argument");

    // Convert &PathBuf to &String
    let ip_str: &str = ip_path.to_str().expect("Invalid UTF-8 in path");
    let ip_string: String = ip_str.to_string();
    
    // Check if it's an IP address or hostname
    if ip_str.parse::<std::net::IpAddr>().is_ok() {
        info!("Parsed IP address: {}", ip_str);
    } else {
        info!("Input is a hostname, attempting DNS resolution...");

        // Attempt DNS resolution and filter only IPv4 addresses
        match (ip_str, 0).to_socket_addrs() {
            Ok(addrs) => {
                let ipv4_addrs: Vec<_> = addrs
                    .filter_map(|addr| {
                        if let IpAddr::V4(ipv4) = addr.ip() {
                            Some(ipv4.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                if !ipv4_addrs.is_empty() {
                    for ipv4 in ipv4_addrs {
                        info!("Resolved IPv4: {}", ipv4);
                    }
                } else {
                    error!("No IPv4 addresses found for the hostname.");
                }
            }
            Err(err) => error!("Failed to resolve hostname: {}", err),
        }
    }
    
    // Attempt to start the receiver
    let receiver = match iris_client::start(&ip_string) {
        Ok(receiver) => receiver,
        Err(e) => {
            error!("Failed to start receiver: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to start receiver").into());
        }
    };

    for _item in receiver {
        
    }

    Ok(())
}