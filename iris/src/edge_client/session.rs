use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{info, error};

use tokio::sync::mpsc::Receiver;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use tonic::transport::Channel;

use super::core::CHANNEL;
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

    pub fn start_report_client(&mut self, mut reports: Receiver<ClientReport>) {

        self.rt.block_on( async move {

            
            let channel = CHANNEL.get().unwrap();

            // Handle the case where the Option contains a value
            let mut connection = ReportServiceClient::new(channel.clone());

            info!("Connected to GRPC Server - Reporting Service");
            while let Some(client_report) = reports.recv().await {
                connection.send_report(client_report).await.unwrap();
            }

                
        });
    }

}
