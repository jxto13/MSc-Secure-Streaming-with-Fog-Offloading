use crate::proto;

use std::sync::{Arc, Mutex};

use chrono::Utc;
use proto::{report_service_server::ReportService, ClientReport};
use log::error;
use tonic::{Request, Response, Status};
use proto::TimestampResponse;

use super::client_sessions::ClientSessions;


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
    async fn report_sink(
        &self,
        request: Request<ClientReport>,
    ) -> Result<Response<TimestampResponse>, Status> {

        // Get the client address from the request metadata
        if let Some(client_addr) = request.remote_addr() {
            let mut lock = self.client_sessions.lock().unwrap();
            // Update the client session
            lock.update_client(client_addr);
            lock.save_client_report(client_addr, request.into_inner());
        } else {
            error!("Client address not found in request metadata");
        }

        Ok(Response::new(TimestampResponse {timestamp: Utc::now().to_rfc3339()}))  // Respond with an empty message
    }
}