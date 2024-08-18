use std::sync::{Arc, Mutex};

use iris::streaming_server::proto::{report_service_server::ReportService, ClientReport, Empty};
use log::{error, info};
use tonic::{Request, Response, Status};

use crate::network_scheduler::client_sessions::ClientSessions;

#[derive(Debug)]
pub struct MyReportService {
    client_sessions: Arc<Mutex<ClientSessions>>,
}

impl MyReportService {
    pub fn new(client_sessions: Arc<Mutex<ClientSessions>>) -> Self {
        Self { 
            client_sessions,
        }
    }
}

#[tonic::async_trait]
impl ReportService for MyReportService {
    async fn send_report(
        &self,
        request: Request<ClientReport>,
    ) -> Result<Response<Empty>, Status> {

        // Get the client address from the request metadata
        if let Some(client_addr) = request.remote_addr() {
            // Update the client session
            self.client_sessions.lock().unwrap().update_client(client_addr);
        } else {
            error!("Client address not found in request metadata");
        }

        let report = request.into_inner();
        info!("Received report: index = {}, payload length = {}", report.index, report.payload.len());

        Ok(Response::new(Empty {}))  // Respond with an empty message
    }
}