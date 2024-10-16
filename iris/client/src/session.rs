use std::sync::Arc;

use chrono::DateTime;
use crossbeam_channel::Receiver;
use log::info;

// use tokio::sync::mpsc::Receiver;
use tokio::runtime::Runtime;

use crate::{CHANNEL, TIME};

use super::proto::report_service_client::ReportServiceClient;
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

        self.rt.block_on( async move {
            
            let channel = CHANNEL.get().unwrap();

            // Handle the case where the Option contains a value
            let mut connection = ReportServiceClient::new(channel.clone());

            info!("Connected to GRPC Server - Reporting Service");
 
            for client_report in reports {
                let response = connection.report_sink(client_report).await.unwrap();
                let timestamp_str = response.into_inner().timestamp;
                {
                    let mut guard = TIME.get().unwrap().lock().unwrap();
                    *guard = DateTime::parse_from_rfc3339(&timestamp_str).unwrap().into();
                }
            }
            
        });
    }

}
