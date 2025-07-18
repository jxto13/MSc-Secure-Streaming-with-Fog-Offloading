use clap::{arg, Command};
use log::{error, info};
use std::error::Error;
use std::io::{self, Read};
use std::net::IpAddr;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
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

    let mut client = iris_client::Client::new();

    let receiver = match client.start(&ip_string) {
        Ok(receiver) => receiver,
        Err(e) => {
            error!("Failed to start receiver: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to start receiver").into());
        }
    };

    for item in receiver {
        if stop_rx.try_recv().is_ok() {
            info!("Stop signal received. Shutting down receiver...");
            client.shutdown();
            break;
        }

        // Process item if needed
    }

    Ok(())
}
