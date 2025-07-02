use std::error::Error;
use std::net::IpAddr;
use std::path::PathBuf;
use log::{error, info};
use std::io;
use clap::{arg, Command};
use std::net::ToSocketAddrs;
use tracing_subscriber::{fmt};

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger
    // env_logger::builder()
    //     .filter_level(log::LevelFilter::Debug)
    //     .format_target(true)
    //     .format_timestamp_millis()
    //     .init();

    tracing_subscriber::fmt()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(tracing::Level::TRACE)
        // sets this to be the default, global collector for this application.
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
    
    // Attempt to start the receiver
    let receiver = match iris_client::start_client(&ip_string) {
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