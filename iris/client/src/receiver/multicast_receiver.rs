use std::net::{Ipv4Addr, SocketAddr};
use std::io;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use log::{error, info};
use mio::{Events, Interest, Poll, Token};
use mio::net::UdpSocket;

// A token to allow us to identify which event is for the `UdpSocket`.
const CLIENT: Token = Token(0);

struct BandwidthMonitor {
    total_bytes: usize,
    packet_count: usize,
    total_packets_received: usize,
    start_time: Instant,
}

impl BandwidthMonitor {
    fn new() -> Self {
        BandwidthMonitor {
            total_bytes: 0,
            packet_count: 0,
            total_packets_received: 0,
            start_time: Instant::now(),
        }
    }

    fn record(&mut self, size: usize) {
        self.total_bytes += size;
        self.packet_count += 1;
        self.total_packets_received += 1;
    }

    fn report(&mut self) {
        let elapsed = self.start_time.elapsed();
        if elapsed >= Duration::from_secs(1) {
            let bitrate_mbps = (self.total_bytes as f64 * 8.0) / 1_000_000.0 / elapsed.as_secs_f64();
            let pps = self.packet_count as f64 / elapsed.as_secs_f64();
            info!("Bitrate: {:.6} Mbit/s, Packets per second: {:.2} pps, Total packets received: {}", bitrate_mbps, pps, self.total_packets_received);

            // Reset counters
            self.total_bytes = 0;
            self.packet_count = 0;
            self.start_time = Instant::now();
        }
    }
}


pub fn start(ip: String, port: String, sender: Sender<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting the multicast receiver");

    // Parse the IP address
    let multicast_address = ip.parse::<Ipv4Addr>().map_err(|e| {
        info!("Error parsing IPv4 address: {}", e);
        e
    }).unwrap();

    info!("Parsed IPv4 address: {}", multicast_address);

    // Parse the port
    let multicast_port = port.parse::<u16>().map_err(|e| {
        info!("Error parsing port: {}", e);
        e
    }).unwrap();

    let multicast_dest: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(multicast_address), multicast_port);

    info!("Binding to multicast destination: {}", multicast_dest);

    // Setup the UDP socket.
    let mut socket = UdpSocket::bind(format!("{}:{}", "0.0.0.0", multicast_port).parse().unwrap()).unwrap();
    info!("Socket bound successfully");

    // Join the multicast group.
    socket.join_multicast_v4(&multicast_address, &Ipv4Addr::UNSPECIFIED).map_err(|err| {
        info!("Failed to join multicast group: {}", err);
        err
    }).unwrap();

    info!("Joined multicast group successfully");
    
    // Create a poll instance.
    let mut poll = Poll::new().unwrap();
    info!("Poll instance created");

    // Register the socket with the poll instance.
    poll.registry().register(&mut socket, CLIENT, Interest::READABLE).unwrap();

    info!("Socket registered with poll instance");

    // Create storage for events.
    let mut events = Events::with_capacity(1024 * 10);

    // Initialize a buffer for the UDP packet.
    let mut buf = [0; 1 << 16]; // Maximum UDP packet size is 65535 bytes.

    // Initialize the bandwidth monitor
    let mut bandwidth_monitor = BandwidthMonitor::new();

    // Start an event loop.
    loop {
        // Poll Mio for events, blocking until we get an event.
        if let Err(err) = poll.poll(&mut events, Some(Duration::from_millis(1))) {
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(Box::new(err));
        }

        // Report bandwidth metrics
        if bandwidth_monitor.total_packets_received != 0 && bandwidth_monitor.total_bytes != 0 {
            bandwidth_monitor.report();
        }

        // Process each event.
        for event in events.iter() {
            match event.token() {
                CLIENT => {
                    loop {
                        // Receive all packets queued for the socket.
                        match socket.recv_from(&mut buf) {
                            Ok((size, _source_address)) => {
                                
                                // Send data to matrix decoder
                                sender.send(buf[..size].to_vec()).unwrap();

                                // Record the size of the received packet
                                bandwidth_monitor.record(size);
                            }
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                break;
                            }
                            Err(e) => {
                                error!("Error receiving packet: {}", e);
                                break;
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    }
}
