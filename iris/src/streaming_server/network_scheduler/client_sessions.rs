use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};
use iris::streaming_server::proto::report_service_server::ReportService;
use iris::streaming_server::proto::{ClientReport, Empty};

use log::{info, error};

use tonic::{Request, Response, Status};

use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct MyReportService {
    client_sessions: Arc<Mutex<ClientSessions>>,
}

impl MyReportService {
    pub fn new(client_sessions: Arc<Mutex<ClientSessions>>) -> Self {
        Self { client_sessions }
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
        // info!("Received report: index = {}, payload length = {}", report.index, report.payload.len());

        // Here you can handle the report (e.g., save it, process it, etc.)

        Ok(Response::new(Empty {}))  // Respond with an empty message
    }
}

#[derive(Debug)]
pub struct ClientSessions {
    clients: Arc<Mutex<HashMap<SocketAddr, Instant>>>,
}

impl ClientSessions {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn update_client(&mut self, client_addr: SocketAddr) {
        let mut clients = self.clients.lock().unwrap();
        clients.insert(client_addr, Instant::now());
    }

    pub fn remove_timed_out_clients(&mut self) {
        let timeout = Duration::from_secs(5);
        let now = Instant::now();
        let mut clients = self.clients.lock().unwrap();
        clients.retain(|_, &mut last_seen| now.duration_since(last_seen) <= timeout);
        // info!("Clients after timeout removal: {:?}", *clients);
    }

    // Getter for the clients hashmap
    pub fn get_clients(&self) -> Arc<Mutex<HashMap<SocketAddr, Instant>>> {
        Arc::clone(&self.clients)
    }
}
