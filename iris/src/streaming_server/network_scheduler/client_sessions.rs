use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::Write;

use crossbeam_channel::{Receiver, Sender};
use iris::streaming_server::proto::report_service_server::ReportService;
use iris::streaming_server::proto::{ClientReport, Empty};

use log::{info, error};
use serde::{Serialize, Deserialize};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct MyReportService {
    client_sessions: Arc<Mutex<ClientSessions>>,
    report_counts: Arc<Mutex<HashMap<i32, i32>>>,
}

impl MyReportService {
    pub fn new(client_sessions: Arc<Mutex<ClientSessions>>) -> Self {
        Self { 
            client_sessions,
            report_counts: Arc::new(Mutex::new(HashMap::new())),
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
        // info!("Received report: index = {}, payload length = {}", report.index, report.payload.len());

        // Save the report to the count hashmap
        if let Err(e) = save_report_to_file(report, &self.report_counts).await {
            error!("Failed to save report: {}", e);
        }

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

#[derive(Serialize, Deserialize)]
struct ReportEntry {
    index: i32,
    reports: i32,
}

async fn save_report_to_file(report: ClientReport, report_counts: &Arc<Mutex<HashMap<i32, i32>>>) -> Result<(), Box<dyn std::error::Error>> {
    // Count the number of zeros in the payload
    let zero_count = report.payload.iter().filter(|&&byte| byte == 0).count() as i32;

    // Update the report counts
    let mut counts = report_counts.lock().unwrap();
    *counts.entry(report.index).or_insert(0) += zero_count;

    // Convert the counts to a vector of ReportEntry
    let mut entries: Vec<ReportEntry> = counts.iter().map(|(&index, &reports)| ReportEntry { index, reports }).collect();

    // Sort the entries by index
    entries.sort_by_key(|entry| entry.index);

    // Path to the JSON file
    let file_path = "./stats/reports.json";

    // Open the file in write mode
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(file_path)?;

    // Serialize the entries to JSON
    let json = serde_json::to_string(&entries)?;

    // Write the JSON to the file
    writeln!(file, "{}", json)?;

    Ok(())
}
