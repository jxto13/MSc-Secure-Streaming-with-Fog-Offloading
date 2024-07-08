use std::sync::Arc;
use std::time::Duration;

use log::{info, error};

use tokio::sync::mpsc::Receiver;
use tokio::runtime::Runtime;
use tokio::time::sleep;

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
            loop {
                let connection = ReportServiceClient::connect("http://razer15:50051").await;
                    
                    match connection {
                        Ok(mut connected_client) => {
                            info!("Connected to GRPC Server - Reporting Service");

                            while let Some(client_report) = reports.recv().await {
                                connected_client.send_report(client_report).await.unwrap();
                            }
                        
                            break; // Exit the loop once connected
                        }
                        Err(e) => {
                            error!("Failed to connect - Reporting Service: {}. Retrying in 3 seconds...", e);
                            sleep(Duration::from_secs(3)).await;
                        }
                    }

            }
        });
    }

}
