use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{info, error};

use tokio::sync::mpsc::Receiver;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use tonic::transport::Channel;

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

    pub fn start_report_client(&mut self, mut reports: Receiver<ClientReport>, channel_arc: Arc<Mutex<Option<Channel>>>) {

        self.rt.block_on( async move {

            let channel_guard = channel_arc.lock().unwrap();

            // Access the Option inside the MutexGuard
            match &*channel_guard {
                Some(channel) => {
                    // Handle the case where the Option contains a value
                    let mut connection = ReportServiceClient::new(channel.clone());

                    info!("Connected to GRPC Server - Reporting Service");

                    // Explicitly release the lock to allow other tasks to access the Mutex, because i cloned the channel
                    drop(channel_guard);
                    while let Some(client_report) = reports.recv().await {
                        connection.send_report(client_report).await.unwrap();
                    }

                },
                None => {
                    // Handle the case where the Option is None
                    error!("No channel available");
                },
            }
                
        });
    }

}
