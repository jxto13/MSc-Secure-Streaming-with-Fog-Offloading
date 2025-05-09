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
            let addr = "192.168.15.1:50053";

            for client_report in reports {
                // Send the UDP packet
                socket.send_to(&compress_data(&client_report.encode_to_vec()), addr).await.unwrap();
            }

        });

    }

}
