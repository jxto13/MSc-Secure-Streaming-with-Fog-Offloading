use std::sync::{Arc, Mutex};

use proto::{report_service_server::ReportService, ClientReport, Empty};
use log::error;
use tonic::{Request, Response, Status};

use crate::{network_scheduler::client_sessions::ClientSessions, proto};

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
            let mut lock = self.client_sessions.lock().unwrap();
            // Update the client session
            lock.update_client(client_addr);
            lock.save_client_report(client_addr, request.into_inner());
        } else {
            error!("Client address not found in request metadata");
        }

        Ok(Response::new(Empty {}))  // Respond with an empty message
    }
}