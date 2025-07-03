mod crypto;
mod membership_manager;
mod metrics;
mod network_scheduler;
mod nts_service;
mod reinforcement_learning;
mod source_handler;

use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use std::{sync::Arc, thread};

// Standard
use crossbeam_channel::unbounded;
use crypto::certificates;
use crypto::key_manager::KeyManager;
use grid::Grid;
use log::{error, info};
use membership_manager::client_sessions;
use membership_manager::reporting_sink::MyReportService;
use once_cell::sync::OnceCell;
use openssl::x509::X509;
use proto::nts_service_server::NtsServiceServer;
use source_handler::data_handler::{DataHandler, DataSource};

use tokio::runtime::Runtime;
use tonic::transport::{Identity, Server, ServerTlsConfig};

use proto::TransmissionPacket;

use iris_server::{read_config, Config};

use crate::metrics::init_metrics;
use crate::metrics::Metrics;
use crate::nts_service::MyNtsService;
use client_sessions::ClientSessions;
use iris_server::DynamicValues;
use tracing_subscriber::field::debug;

// Import protobuff
pub mod proto {
    include!("../proto/generated/streaming_server.rs");
}

// Define the global static instance of Config using OnceCell
static CONFIG: OnceCell<Config> = OnceCell::new();
static MANAGER: OnceCell<Arc<RwLock<KeyManager>>> = OnceCell::new();
static MATRIX_BUFFER: OnceCell<Arc<MatrixHistory>> = OnceCell::new();
static METRICS: OnceCell<Arc<Metrics>> = OnceCell::new();
static DYNAMIC_VALUES: OnceCell<Arc<RwLock<DynamicValues>>> = OnceCell::new();

// Custom
use clap::{arg, Command};
use network_scheduler::pipeline::Pipeline;

/// Start the Prometheus exporter
/// This function binds the exporter to the address specified in the configuration file
fn start_prometheus_exporter() {
    // Parse address used to bind exporter to.
    let prometheus_port = &CONFIG.get().unwrap().prometheus.port;
    let socket_addr = format!("0.0.0.0:{}", prometheus_port).parse().unwrap();

    // Start exporter and update metrics every five seconds.
    prometheus_exporter::start(socket_addr).expect("can not start exporter");

    // Initialize the metrics
    init_metrics();
}

/// Periodically clean up client sessions using `tokio::time::interval`
async fn clean_client_session(client_sessions: Arc<ClientSessions>) {
    let config = CONFIG.get().unwrap();
    let mut interval =
        tokio::time::interval(Duration::from_millis(config.settings.timeout_clients));

    loop {
        // Wait for the next tick
        interval.tick().await;

        // Clean up timed out clients
        client_sessions.remove_timed_out_clients();
    }
}

/// Periodically update packet loss metrics using `tokio::time::interval`
async fn update_metrics(client_sessions: Arc<ClientSessions>) {
    let mut interval = tokio::time::interval(Duration::from_millis(1000));

    loop {
        // Wait for the next tick
        interval.tick().await;

        // Update the metrics for client sessions
        client_sessions.update_metrics();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the thread handlers
    let mut handlers = vec![];

    // Set a custom panic hook to log panics and exit gracefully
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => s.as_str(),
                None => "Unknown panic message",
            },
        };

        eprintln!("\n==== Panic occurred! ====");
        eprintln!("Message  : {}", message);
        eprintln!("Location : {}", location);

        // Optionally print backtrace if enabled
        if std::env::var("RUST_BACKTRACE").unwrap_or_default() == "1" {
            eprintln!(
                "Backtrace:\n{:?}",
                std::backtrace::Backtrace::force_capture()
            );
        } else {
            eprintln!("(Enable backtrace with `RUST_BACKTRACE=1`)");
        }

        eprintln!("=========================\n");

        std::process::exit(1);
    }));

    // Define the command structure
    let cmd = Command::new("streaming_server")
        .version("0.1.0")
        .author("Joaquim Oliveira <up201908075@up.pt>")
        .about("MSc Secure Streaming with Fog Offloading")
        .arg(
            arg!(--"config" <FILE>)
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("Path to the configuration file"),
        )
        .arg(
            arg!(--"train")
                .required(false)
                .help("Enable training mode for RL simulation")
                .action(clap::ArgAction::SetTrue),
        );

    // Parse command-line arguments
    let matches = cmd.get_matches();
    let config_filename = matches
        .get_one::<PathBuf>("config")
        .expect("required argument");

    // Init the logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()
        .init();

    // tracing_subscriber::fmt()
    //     // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
    //     // will be written to stdout.
    //     .with_max_level(tracing::Level::TRACE)
    //     // sets this to be the default, global collector for this application.
    //     .init();

    if matches.get_flag("train") {
        info!("------------------------- Training mode enabled  -------------------------");
    } else {
        info!("------------------------- Training mode disabled -------------------------");
    }

    // Read configuration file
    let config = read_config(config_filename.to_str().unwrap())?;

    // Set the global CONFIG instance
    CONFIG.set(config).expect("Failed to set CONFIG");

    MATRIX_BUFFER
        .set(Arc::new(MatrixHistory::new(
            64,
            Duration::from_millis(CONFIG.get().unwrap().retransmission.deadline),
        )))
        .expect("Failed to set MATRIX_BUFFER");

    // Start the Prometheus exporter
    start_prometheus_exporter();

    // Initialize the dynamic values
    let dynamic_values = Arc::new(RwLock::new(DynamicValues {
        fec_row_redundancy: CONFIG.get().unwrap().multicast.row_fec,
        fec_col_redundancy: CONFIG.get().unwrap().multicast.col_fec,
        data_row_size: CONFIG.get().unwrap().multicast.row,
        data_col_size: CONFIG.get().unwrap().multicast.col,
        block_size: CONFIG.get().unwrap().multicast.shard_size,
    }));

    // Set the global DYNAMIC_VALUES instance
    DYNAMIC_VALUES
        .set(dynamic_values)
        .expect("Failed to set DYNAMIC_VALUES");

    // Set up a Tokio runtime
    let rt = Arc::new(Runtime::new().unwrap());

    // ----------------------------------- Programs Setup -------------------------------------------------------------

    // Load or create certificates
    let certificates = certificates::load_or_create_certificates().unwrap();
    let key_manager = Arc::new(RwLock::new(KeyManager::new(certificates.ec_key)));

    // Start key_service before the pipeline to generate the ECDSA Keys for encoder
    let key_service = crypto::key_service::get_service(
        Arc::clone(&key_manager),
        X509::from_pem(certificates.ec_cert.as_bytes()).unwrap(),
    );

    MANAGER.set(key_manager).unwrap();

    // ----------------------------------- Programs Starts Here ------------------------------------------------------

    // Create channels
    let (source_handler_sender, source_handler_receiver) = unbounded();
    let (reports_channel_sender, reports_channel_receiver) = unbounded();

    // Choose the data source (THIS WILL SPAWN THREADS INSIDE *not handled by the main thread*)
    let source = match CONFIG.get().unwrap().multicast.data_source.as_str() {
        "mpegts" => DataSource::MPEGTS(CONFIG.get().unwrap().multicast.ip_source.clone()),
        _ => panic!("Invalid data_source value in config"),
    };

    handlers.push((
        "source_handler",
        thread::spawn(move || {
            // Create a DataHandler instance based on datasource
            DataHandler::new(source, source_handler_sender).start();
        }),
    ));

    // Create the client sessions
    let client_sessions = Arc::new(ClientSessions::new(reports_channel_receiver));

    // Start client_sessions thread
    handlers.push((
        "client_sessions_handler",
        thread::spawn({
            let client_sessions = Arc::clone(&client_sessions);
            move || {
                // Start the client sessions manager, this processes incoming reports
                client_sessions.start_manager();
            }
        }),
    ));

    // Create the pipeline
    handlers.push((
        "pipeline_handler",
        thread::spawn({
            let client_sessions = Arc::clone(&client_sessions);
            move || {
                Pipeline::start(client_sessions, source_handler_receiver);
            }
        }),
    ));

    // Creates the reporting sink
    handlers.push((
        "reporting_sink_handler",
        thread::spawn({
            let client_sessions = Arc::clone(&client_sessions);
            move || {
                // Start the UDP receiver for reports
                MyReportService::receiver(client_sessions, reports_channel_sender);
            }
        }),
    ));

    // Create a thread to handle stdin input (will be removed in production)
    handlers.push((
        "stdin_handler",
        thread::spawn(|| {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            let metrics = metrics::get_metrics();
            for line in stdin.lock().lines() {
                match line {
                    Ok(input) => {
                        let parts: Vec<&str> = input.trim().split_whitespace().collect();
                        if parts.len() != 2 {
                            eprintln!("Please enter exactly two numbers separated by a space.");
                            continue;
                        }

                        let a = parts[0].parse::<i32>();
                        let b = parts[1].parse::<i32>();

                        match (a, b) {
                            (Ok(x), Ok(y)) => {
                                metrics.fec_row.set(x as i64);
                                metrics.fec_col.set(y as i64);
                            }
                            _ => eprintln!("Invalid input. Make sure both entries are integers."),
                        }
                    }
                    Err(e) => eprintln!("Error reading line: {}", e),
                }
            }
        }),
    ));

    handlers.push((
        "reinforcement_learning_handler",
        thread::spawn({
            let client_sessions = Arc::clone(&client_sessions);
            move || {

                // use crate::reinforcement_learning::agent::QLearningAgent;
                // use crate::reinforcement_learning::action::Direction;
                // use crate::reinforcement_learning::action::FecDataAction;
                // use crate::reinforcement_learning::state::State;

                // let directions = [Direction::Increase, Direction::Decrease, Direction::NoChange];
                // let mut actions = Vec::new();

                // // Get the dynamic values from the global static instance
                // let dynamic_values = DYNAMIC_VALUES.get().unwrap();

                // // Read the dynamic values with a read lock
                // let dynamic_values_read = dynamic_values.read().unwrap();

                // // Initialize current sizes (should come from your system state)
                // let mut current_fec_row = dynamic_values_read.fec_row_redundancy;
                // let mut current_fec_col = dynamic_values_read.fec_col_redundancy;

                // // Pull data matrix size from static config
                // let config = CONFIG.get().unwrap();
                // let data_row = config.multicast.row;
                // let data_col = config.multicast.col;

                // // drop the read lock to allow other threads to modify dynamic values
                // drop(dynamic_values_read);

                // // Generate all combinations of actions
                // for &fec_row_dir in &directions {
                //     for &fec_col_dir in &directions {

                //         // Calculate candidate new sizes
                //         let new_fec_row = adjust_size(current_fec_row, fec_row_dir);
                //         let new_fec_col = adjust_size(current_fec_col, fec_col_dir);

                //         // Check bandwidth constraint
                //         let overhead_ratio = (new_fec_row + new_fec_col) as f64 / (data_row * data_col) as f64;
                //         if overhead_ratio <= 0.4 {
                //             actions.push(FecDataAction {
                //                 fec_row: fec_row_dir,
                //                 fec_col: fec_col_dir,
                //             });
                //         }
                //     }

                // }

                // fn adjust_size(size: usize, dir: Direction) -> usize {
                //     match dir {
                //         Direction::Increase => size + 1,
                //         Direction::Decrease => size.saturating_sub(1).max(1),
                //         Direction::NoChange => size,
                //     }
                // }

                // let mut agent = QLearningAgent::new(actions, 0.1, 0.9, 0.2);

                // fn measure_reward(
                //     state: &State,
                //     action: &FecDataAction,
                //     current_fec_row: usize,
                //     current_fec_col: usize,
                //     data_row: usize,
                //     data_col: usize
                // ) -> f64 {
                //     let new_fec_row = adjust_size(current_fec_row, action.fec_row);
                //     let new_fec_col = adjust_size(current_fec_col, action.fec_col);

                //     let overhead_ratio = (new_fec_row + new_fec_col) as f64 / (data_row * data_col) as f64;
                //     let cost = overhead_ratio * 10.0; // penalize FEC usage

                //     -1.5 * (state.loss_bin as f64) - cost
                // }

                // loop {
                //     let state = client_sessions.get_session_state();
                //     let action = agent.choose_action(state.clone());

                //     // Use current values for reward BEFORE applying action
                //     let reward = measure_reward(&state, &action, current_fec_row, current_fec_col, data_row, data_col);

                //     let new_fec_row = adjust_size(current_fec_row, action.fec_row);
                //     let new_fec_col = adjust_size(current_fec_col, action.fec_col);

                //     {
                //         let mut dynamic_values = DYNAMIC_VALUES.get().unwrap().write().unwrap();
                //         dynamic_values.fec_row_redundancy = new_fec_row;
                //         dynamic_values.fec_col_redundancy = new_fec_col;
                //     }

                //     log::debug!("State: {:?}, Action: {:?}, Reward: {}", state, action, reward);

                //     std::thread::sleep(std::time::Duration::from_secs(5));

                //     let next_state = client_sessions.get_session_state();
                //     agent.update(state, action.clone(), reward, next_state);

                //     // Update current state after action
                //     current_fec_row = new_fec_row;
                //     current_fec_col = new_fec_col;
                // }

                // reinforcement_learning::beta::start(&client_sessions);
            }
        }),
    ));

    let client_sessions_clone = Arc::clone(&client_sessions);

    // Run async code using the Tokio runtime
    rt.spawn(async {
        // Listen to all IPV4 addresses
        let addr = format!("0.0.0.0:{}", CONFIG.get().unwrap().grpc.port)
            .parse()
            .unwrap();

        // Start grpc server this will block execution because of await
        Server::builder()
            .tls_config(ServerTlsConfig::new().identity(Identity::from_pem(
                certificates.server_cert,
                certificates.server_key,
            )))
            .unwrap()
            .add_service(crypto::ca_service::get_service(
                certificates.ca_cert,
                certificates.ca_private_key,
                client_sessions_clone,
            )) // Responsible for signing the client certificates
            .add_service(key_service)
            .add_service(NtsServiceServer::new(MyNtsService::default()))
            .serve(addr)
            .await
            .unwrap_or_else(|err| {
                // Handle the error if creating the runtime fails
                panic!("Error creating gRPC server: {}", err);
            });
    });

    // Start the periodic task
    rt.spawn(clean_client_session(Arc::clone(&client_sessions)));
    rt.spawn(update_metrics(Arc::clone(&client_sessions)));

    // Wait for all threads to finish
    for (name, handler) in handlers {
        if let Err(e) = handler.join() {
            panic!("{} panicked: {:?}", name, e);
        }
    }

    Ok(())
}

/// ------------------------------------------ Matrix History -------------------------------------------------------------
/// A struct to hold the matrix data and its index
#[derive(Clone, Debug)]
pub struct IndexedMatrix {
    pub index: usize,
    pub data: Grid<TransmissionPacket>,
    pub created_at: Instant,
}

#[derive(Debug)]
pub struct MatrixHistory {
    matrices: Vec<RwLock<Option<IndexedMatrix>>>,
    capacity: usize,
    latest_index: RwLock<Option<usize>>,
    deadline: Duration,
}
impl MatrixHistory {
    pub fn new(capacity: usize, deadline: Duration) -> Self {
        MatrixHistory {
            matrices: (0..capacity).map(|_| RwLock::new(None)).collect(),
            capacity,
            latest_index: RwLock::new(None),
            deadline,
        }
    }

    pub fn is_valid_index(&self, index: usize) -> bool {
        if let Some(latest) = *self.latest_index.read().unwrap() {
            index >= latest.saturating_sub(self.capacity - 1) && index <= latest
        } else {
            false
        }
    }

    pub fn add_matrix(&self, index: usize, grid: Grid<TransmissionPacket>) {
        let slot_index = index % self.capacity;
        let slot = &self.matrices[slot_index];
        let mut slot_guard = slot.write().unwrap();

        *slot_guard = Some(IndexedMatrix {
            index,
            data: grid,
            created_at: Instant::now(),
        });

        let mut latest_guard = self.latest_index.write().unwrap();
        *latest_guard = Some(index);
    }

    pub fn get_matrix(&self, index: usize) -> Option<Grid<TransmissionPacket>> {
        let slot_index = index % self.capacity;
        let slot = &self.matrices[slot_index];
        let guard = slot.read().unwrap();

        if let Some(ref matrix) = *guard {
            if matrix.index == index {
                return Some(matrix.data.clone());
            }
        }
        None
    }
}
