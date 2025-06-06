use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use crossbeam_channel::Receiver;

use prost::Message;
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;

use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

use super::proto::ClientReport;

pub struct ClientSession {
    rt: Arc<Runtime>
}

impl ClientSession {
    pub fn new(runtime : Arc<Runtime>) -> Self {
        Self {
            rt: runtime
        }
    }

    fn first_ipv4(host: &str, port: u16) -> Option<Ipv4Addr> {
        let addr_iter = (host, port).to_socket_addrs().ok()?;

        for addr in addr_iter {
            match addr {
                SocketAddr::V4(v4_addr) => return Some(*v4_addr.ip()),
                SocketAddr::V6(_) => continue,
            }
        }

        None
    }

    // function to receive reports and send based on a time window based on the timeclock of the server
    
    pub fn start_report_client(&mut self, reports: Receiver<ClientReport>) {

        fn compress_data(data: &[u8]) -> Vec<u8> {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data).unwrap();
            encoder.finish().unwrap()
        }

        // udp report sink

        self.rt.block_on( async move {
            // Create a UDP socket
            let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
            
            // Destination address
            let hostname = "stream1.iris.dcc.fc.up.pt";

            for client_report in reports {
                // Send the UDP packet
                socket.send_to(&compress_data(&client_report.encode_to_vec()), format!("{}:50053",hostname)).await.unwrap();
            }

        });

    }

}
