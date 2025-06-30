use std::net::Ipv4Addr;

use chrono::{DateTime, FixedOffset};

use openssl::ec::EcKey;
use openssl::pkey::Private;

use crate::proto::SessionKey;

#[derive(Debug, Clone, Default)]
pub struct ClientSession {
    pub current_key: SessionKey,
    pub next_key: SessionKey,
    pub ec_server_certificate: Vec<u8>,
    pub client_ec_certificate: Vec<u8>,
    pub client_ec_key: Option<EcKey<Private>>,
    pub server_time: DateTime<FixedOffset>,
    pub epoch: u32,
    pub client_reporting_interval: u32,
    pub client_sliding_window: u32,
    pub server_ip: String,
    pub retransmission_port: u32,
    pub unicast_port: u32,
    pub multicast_address: String, // This is the address for multicast packets
    pub multicast_port: u32,
}

// impl ClientSession {
//     pub fn new(runtime : Arc<Runtime>) -> Self {
//         Self {
//             rt: runtime
//         }
//     }

//     // function to receive reports and send based on a time window based on the timeclock of the server
    
//     pub fn start_report_client(&mut self, reports: Receiver<ClientReport>) {

//         // udp report sink

//         self.rt.spawn( async {
//             // Create a UDP socket
//             let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
            
//             // Destination address
//             let hostname = "stream1.iris.dcc.fc.up.pt";

//             for client_report in reports {
//                 // Send the UDP packet
//                 socket.send_to(&compress_data(&client_report.encode_to_vec()), format!("{}:50053",hostname)).await.unwrap();
//             }

//         });

//     }

// }
