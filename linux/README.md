# Linux Client Implementation

## Overview

The Linux client implementation provides a native, high-performance streaming client for Linux-based systems. Built entirely in Rust, this component demonstrates the cross-platform capabilities of the IRIS streaming protocol while serving as a reference implementation for desktop and server environments.

## Architecture

### System Design

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  IRIS Server    │    │  Linux Client   │    │  Video Output   │
│                 │    │                 │    │                 │
│  • gRPC API     │◄──►│  • Stream Recv  │───►│  • Playback     │
│  • Multicast    │    │  • FEC Decoding │    │  • File Save    │
│  • Key Service  │    │  • Certificate  │    │  • Analysis     │
│  • Reporting    │    │  • Reporting    │    │  • Metrics      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Technical Stack

- **Rust**: Core implementation language for performance and safety
- **Tokio**: Asynchronous runtime for concurrent operations
- **Tonic/gRPC**: Server communication and control plane
- **Reed-Solomon**: Forward Error Correction implementation
- **OpenSSL**: Cryptographic operations and certificate handling

## Core Implementation

### Project Structure

```
linux/
├── Cargo.toml           # Project dependencies and configuration
├── src/
│   └── main.rs         # Main application entry point
└── README.md           # This documentation
```

### Dependencies

```toml
[package]
name = "iris-linux-client"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core IRIS functionality
iris-client = { version = "0.1.0", path = "../iris/client" }

# Async runtime and networking
tokio = { version = "1.45.0", features = ["rt-multi-thread", "net", "fs"] }
mio = { version = "1.0.2", features = ["os-poll", "net"] }

# Command line interface
clap = { version = "4.5.0", features = ["derive"] }

# Logging and error handling
log = "0.4.22"
env_logger = "0.11.0"
anyhow = "1.0"

# Serialization and networking
serde = { version = "1.0", features = ["derive"] }
crossbeam-channel = "0.5.13"
```

## Application Architecture

### Main Application Loop

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Create and configure client
    let client = IrisClient::new(ClientConfig {
        server_address: args.server,
        stream_url: args.stream,
        output_path: args.output,
        quality: args.quality,
    }).await?;
    
    // Start streaming
    client.start_streaming().await?;
    
    Ok(())
}
```

### Command Line Interface

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IRIS server address
    #[arg(short, long, default_value = "https://stream1.iris.dcc.fc.up.pt:50051")]
    server: String,
    
    /// Stream URL to request
    #[arg(short = 'u', long)]
    stream: String,
    
    /// Output file path (optional, defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Requested quality level
    #[arg(short, long, default_value = "medium")]
    quality: String,
    
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
    
    /// Certificate file path
    #[arg(short, long)]
    cert: Option<PathBuf>,
}
```

### Client Configuration

```rust
pub struct ClientConfig {
    pub server_address: String,
    pub stream_url: String,
    pub output_path: Option<PathBuf>,
    pub quality: String,
    pub certificate_path: Option<PathBuf>,
    pub buffer_size: usize,
    pub timeout: Duration,
    pub retry_attempts: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_address: "https://stream1.iris.dcc.fc.up.pt:50051".to_string(),
            stream_url: String::new(),
            output_path: None,
            quality: "medium".to_string(),
            certificate_path: None,
            buffer_size: 8192,
            timeout: Duration::from_secs(30),
            retry_attempts: 3,
        }
    }
}
```

## Streaming Implementation

### Client State Management

```rust
pub struct IrisClient {
    config: ClientConfig,
    grpc_client: StreamingClient<Channel>,
    multicast_receiver: MulticastReceiver,
    fec_decoder: FECDecoder,
    output_writer: OutputWriter,
    metrics_collector: MetricsCollector,
    running: Arc<AtomicBool>,
}

impl IrisClient {
    pub async fn new(config: ClientConfig) -> Result<Self> {
        // Initialize gRPC client with TLS
        let channel = Channel::from_shared(config.server_address.clone())?
            .tls_config(ClientTlsConfig::new())?
            .connect()
            .await?;
        
        let grpc_client = StreamingClient::new(channel);
        
        // Initialize components
        let multicast_receiver = MulticastReceiver::new(&config).await?;
        let fec_decoder = FECDecoder::new();
        let output_writer = OutputWriter::new(config.output_path.clone())?;
        let metrics_collector = MetricsCollector::new();
        
        Ok(Self {
            config,
            grpc_client,
            multicast_receiver,
            fec_decoder,
            output_writer,
            metrics_collector,
            running: Arc::new(AtomicBool::new(false)),
        })
    }
}
```

### Stream Processing Pipeline

```rust
impl IrisClient {
    pub async fn start_streaming(&self) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        
        // Request session key from server
        let key_response = self.request_session_key().await?;
        
        // Start multicast receiver
        let (packet_tx, packet_rx) = mpsc::channel(1000);
        let receiver_handle = self.start_multicast_receiver(packet_tx).await?;
        
        // Start packet processing pipeline
        let processor_handle = self.start_packet_processor(packet_rx, key_response).await?;
        
        // Start metrics reporting
        let metrics_handle = self.start_metrics_reporting().await?;
        
        // Wait for completion or interruption
        tokio::select! {
            _ = receiver_handle => log::warn!("Multicast receiver ended"),
            _ = processor_handle => log::warn!("Packet processor ended"),
            _ = metrics_handle => log::warn!("Metrics reporter ended"),
            _ = tokio::signal::ctrl_c() => {
                log::info!("Received interrupt signal, shutting down");
                self.running.store(false, Ordering::SeqCst);
            }
        }
        
        Ok(())
    }
}
```

### Multicast Reception

```rust
pub struct MulticastReceiver {
    socket: UdpSocket,
    buffer: Vec<u8>,
    multicast_addr: SocketAddr,
}

impl MulticastReceiver {
    pub async fn new(config: &ClientConfig) -> Result<Self> {
        let multicast_addr: SocketAddr = "224.0.0.251:8080".parse()?;
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        
        // Join multicast group
        socket.join_multicast_v4(
            multicast_addr.ip().clone().try_into()?,
            Ipv4Addr::UNSPECIFIED
        )?;
        
        Ok(Self {
            socket,
            buffer: vec![0u8; config.buffer_size],
            multicast_addr,
        })
    }
    
    pub async fn receive_packet(&mut self) -> Result<PacketWrapper> {
        let (len, _) = self.socket.recv_from(&mut self.buffer).await?;
        let packet = PacketWrapper::decode(&self.buffer[..len])?;
        Ok(packet)
    }
}
```

## Forward Error Correction

### FEC Decoder Implementation

```rust
use reed_solomon_16::*;

pub struct FECDecoder {
    matrices: HashMap<u32, Matrix>,
    decoder: Decoder,
}

impl FECDecoder {
    pub fn new() -> Self {
        Self {
            matrices: HashMap::new(),
            decoder: Decoder::new(8, 4), // 8 data + 4 parity shards
        }
    }
    
    pub fn process_packet(&mut self, packet: TransmissionPacket) -> Result<Option<Vec<u8>>> {
        let matrix = self.matrices.entry(packet.matrix)
            .or_insert_with(|| Matrix::new(packet.rows, packet.cols));
        
        // Store packet in matrix
        matrix.set_packet(packet.index, packet.payload)?;
        
        // Attempt reconstruction if enough packets received
        if matrix.can_reconstruct() {
            let reconstructed = self.decoder.reconstruct_data(matrix.data())?;
            Ok(Some(reconstructed))
        } else {
            Ok(None)
        }
    }
}

struct Matrix {
    data: Vec<Option<Vec<u8>>>,
    rows: usize,
    cols: usize,
    received_count: usize,
    required_count: usize,
}

impl Matrix {
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            data: vec![None; rows * cols],
            rows,
            cols,
            received_count: 0,
            required_count: rows, // Need all data shards
        }
    }
    
    fn set_packet(&mut self, index: u32, data: Vec<u8>) -> Result<()> {
        let idx = index as usize;
        if idx < self.data.len() && self.data[idx].is_none() {
            self.data[idx] = Some(data);
            self.received_count += 1;
        }
        Ok(())
    }
    
    fn can_reconstruct(&self) -> bool {
        self.received_count >= self.required_count
    }
}
```

## Security Integration

### Certificate Management

```rust
use openssl::x509::X509;
use openssl::pkey::PKey;

pub struct CertificateManager {
    server_cert: Option<X509>,
    client_cert: Option<X509>,
    private_key: Option<PKey<openssl::pkey::Private>>,
}

impl CertificateManager {
    pub fn new() -> Self {
        Self {
            server_cert: None,
            client_cert: None,
            private_key: None,
        }
    }
    
    pub async fn request_certificate(&mut self, grpc_client: &mut StreamingClient<Channel>) -> Result<()> {
        // Generate CSR
        let (private_key, csr) = self.generate_csr()?;
        
        // Request certificate from server
        let request = CertificateRequest {
            csr: csr.to_der()?,
        };
        
        let response = grpc_client.sign_certificate(request).await?;
        
        // Store certificate and key
        self.client_cert = Some(X509::from_der(&response.into_inner().signed_cert)?);
        self.private_key = Some(private_key);
        
        Ok(())
    }
    
    fn generate_csr(&self) -> Result<(PKey<openssl::pkey::Private>, X509Req)> {
        // Generate key pair
        let rsa = openssl::rsa::Rsa::generate(2048)?;
        let private_key = PKey::from_rsa(rsa)?;
        
        // Create certificate signing request
        let mut req_builder = X509ReqBuilder::new()?;
        req_builder.set_pubkey(&private_key)?;
        
        // Set subject name
        let mut name = X509NameBuilder::new()?;
        name.append_entry_by_text("CN", "iris-linux-client")?;
        req_builder.set_subject_name(&name.build())?;
        
        // Sign the request
        req_builder.sign(&private_key, openssl::hash::MessageDigest::sha256())?;
        
        Ok((private_key, req_builder.build()))
    }
}
```

### Packet Verification

```rust
use openssl::hash::{hash, MessageDigest};
use openssl::sign::Verifier;

impl IrisClient {
    fn verify_packet_signature(&self, packet: &TransmissionPacket) -> Result<bool> {
        // Extract packet signature
        let signature = &packet.packet_signature;
        
        // Create packet hash
        let mut hasher = openssl::hash::Hasher::new(MessageDigest::sha256())?;
        hasher.update(&packet.matrix.to_le_bytes())?;
        hasher.update(&packet.index.to_le_bytes())?;
        hasher.update(&packet.payload)?;
        let hash = hasher.finish()?;
        
        // Verify signature using server's public key
        if let Some(cert) = &self.cert_manager.server_cert {
            let public_key = cert.public_key()?;
            let mut verifier = Verifier::new(MessageDigest::sha256(), &public_key)?;
            verifier.update(&hash)?;
            Ok(verifier.verify(signature)?)
        } else {
            Ok(false)
        }
    }
}
```

## Output Management

### Video Output Writer

```rust
use std::fs::File;
use std::io::{Write, BufWriter};

pub struct OutputWriter {
    writer: Box<dyn Write + Send>,
    bytes_written: u64,
}

impl OutputWriter {
    pub fn new(output_path: Option<PathBuf>) -> Result<Self> {
        let writer: Box<dyn Write + Send> = match output_path {
            Some(path) => {
                let file = File::create(path)?;
                Box::new(BufWriter::new(file))
            }
            None => Box::new(std::io::stdout()),
        };
        
        Ok(Self {
            writer,
            bytes_written: 0,
        })
    }
    
    pub async fn write_frame(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        self.bytes_written += data.len() as u64;
        Ok(())
    }
    
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}
```

### Format Detection and Processing

```rust
pub struct VideoProcessor {
    format_detector: FormatDetector,
    frame_parser: FrameParser,
}

impl VideoProcessor {
    pub fn process_frame(&mut self, data: &[u8]) -> Result<ProcessedFrame> {
        // Detect format if not already known
        if !self.format_detector.is_format_known() {
            self.format_detector.analyze(data)?;
        }
        
        // Parse frame based on detected format
        let frame = self.frame_parser.parse_frame(data)?;
        
        Ok(ProcessedFrame {
            timestamp: frame.timestamp,
            frame_type: frame.frame_type,
            data: frame.data,
            quality_metrics: self.calculate_quality_metrics(&frame),
        })
    }
    
    fn calculate_quality_metrics(&self, frame: &Frame) -> QualityMetrics {
        QualityMetrics {
            resolution: frame.resolution,
            bitrate: frame.estimated_bitrate,
            frame_rate: frame.frame_rate,
            quality_score: self.estimate_quality_score(frame),
        }
    }
}
```

## Performance Monitoring

### Metrics Collection

```rust
use std::time::{Instant, Duration};

pub struct MetricsCollector {
    start_time: Instant,
    packets_received: u64,
    packets_lost: u64,
    bytes_received: u64,
    frames_decoded: u64,
    last_report_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            packets_received: 0,
            packets_lost: 0,
            bytes_received: 0,
            frames_decoded: 0,
            last_report_time: now,
        }
    }
    
    pub fn record_packet_received(&mut self, size: usize) {
        self.packets_received += 1;
        self.bytes_received += size as u64;
    }
    
    pub fn record_packet_lost(&mut self) {
        self.packets_lost += 1;
    }
    
    pub fn record_frame_decoded(&mut self) {
        self.frames_decoded += 1;
    }
    
    pub fn get_statistics(&self) -> ClientStatistics {
        let elapsed = self.start_time.elapsed();
        let packet_loss_rate = if self.packets_received + self.packets_lost > 0 {
            self.packets_lost as f64 / (self.packets_received + self.packets_lost) as f64
        } else {
            0.0
        };
        
        ClientStatistics {
            uptime: elapsed,
            packets_received: self.packets_received,
            packets_lost: self.packets_lost,
            packet_loss_rate,
            average_bitrate: (self.bytes_received * 8) as f64 / elapsed.as_secs_f64(),
            frames_per_second: self.frames_decoded as f64 / elapsed.as_secs_f64(),
        }
    }
}
```

### Performance Reporting

```rust
impl IrisClient {
    async fn start_metrics_reporting(&self) -> Result<JoinHandle<()>> {
        let mut grpc_client = self.grpc_client.clone();
        let metrics_collector = self.metrics_collector.clone();
        let running = self.running.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            while running.load(Ordering::SeqCst) {
                interval.tick().await;
                
                let stats = metrics_collector.get_statistics();
                let report = ClientReport {
                    client_id: get_client_id(),
                    counter: get_report_counter(),
                    metrics: ReportMetrics {
                        multicast_retransmissions: 0, // Linux client doesn't use mesh
                        unicast_retransmissions: 0,
                        wifi_aware_retransmissions: 0,
                    },
                    reports: vec![], // Matrix reports would go here
                    signature: vec![], // Sign the report
                };
                
                match grpc_client.report_sink(report).await {
                    Ok(_) => log::debug!("Metrics report sent successfully"),
                    Err(e) => log::error!("Failed to send metrics report: {}", e),
                }
            }
        });
        
        Ok(handle)
    }
}
```

## Command Line Usage

### Basic Usage Examples

```bash
# Connect to default server and stream to stdout
iris-linux-client -u "rtmp://input.stream/live"

# Stream to file with specific quality
iris-linux-client -u "rtmp://input.stream/live" -o output.mp4 -q high

# Use custom server and certificate
iris-linux-client -s "https://custom.server:50051" \
                  -u "rtmp://stream/live" \
                  -c /path/to/cert.pem \
                  --debug

# Stream with specific buffer size and timeout
iris-linux-client -u "rtmp://stream/live" \
                  --buffer-size 16384 \
                  --timeout 60
```

### Configuration Options

```bash
Options:
  -s, --server <SERVER>      IRIS server address [default: https://stream1.iris.dcc.fc.up.pt:50051]
  -u, --stream <STREAM>      Stream URL to request
  -o, --output <OUTPUT>      Output file path (optional, defaults to stdout)
  -q, --quality <QUALITY>    Requested quality level [default: medium]
  -c, --cert <CERT>          Certificate file path
  -d, --debug                Enable debug logging
      --buffer-size <SIZE>   Network buffer size [default: 8192]
      --timeout <SECONDS>    Connection timeout [default: 30]
      --retry <COUNT>        Retry attempts [default: 3]
  -h, --help                 Print help information
  -V, --version              Print version information
```

## Error Handling and Recovery

### Robust Error Handling

```rust
use anyhow::{Context, Result, anyhow};

impl IrisClient {
    async fn handle_connection_error(&self, error: &dyn std::error::Error) -> Result<()> {
        match error.downcast_ref::<tonic::Status>() {
            Some(status) => match status.code() {
                tonic::Code::Unavailable => {
                    log::warn!("Server unavailable, attempting reconnection");
                    self.reconnect_with_backoff().await?;
                }
                tonic::Code::Unauthenticated => {
                    log::warn!("Authentication failed, requesting new certificate");
                    self.renew_certificate().await?;
                }
                _ => {
                    log::error!("gRPC error: {}", status);
                    return Err(anyhow!("Unrecoverable gRPC error"));
                }
            }
        }
        _ => {
            log::error!("Unknown error: {}", error);
            return Err(anyhow!("Unknown error occurred"));
        }
        
        Ok(())
    }
    
    async fn reconnect_with_backoff(&self) -> Result<()> {
        let mut delay = Duration::from_millis(100);
        let max_delay = Duration::from_secs(30);
        
        for attempt in 1..=self.config.retry_attempts {
            log::info!("Reconnection attempt {} of {}", attempt, self.config.retry_attempts);
            
            match self.establish_connection().await {
                Ok(_) => {
                    log::info!("Reconnection successful");
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Reconnection attempt {} failed: {}", attempt, e);
                    if attempt < self.config.retry_attempts {
                        tokio::time::sleep(delay).await;
                        delay = std::cmp::min(delay * 2, max_delay);
                    }
                }
            }
        }
        
        Err(anyhow!("Failed to reconnect after {} attempts", self.config.retry_attempts))
    }
}
```

### Stream Recovery

```rust
impl IrisClient {
    async fn handle_stream_interruption(&self) -> Result<()> {
        log::warn!("Stream interruption detected, attempting recovery");
        
        // Clear partial data
        self.fec_decoder.clear_partial_matrices();
        
        // Request stream restart
        let restart_request = StreamRestartRequest {
            stream_url: self.config.stream_url.clone(),
            last_received_sequence: self.get_last_sequence_number(),
        };
        
        let response = self.grpc_client.restart_stream(restart_request).await?;
        
        // Update decryption keys if provided
        if let Some(new_key) = response.into_inner().session_key {
            self.update_session_key(new_key).await?;
        }
        
        log::info!("Stream recovery completed");
        Ok(())
    }
}
```

## Performance Optimizations

### Memory Management

```rust
use std::sync::Arc;
use std::collections::VecDeque;

pub struct BufferPool {
    buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    buffer_size: usize,
    max_buffers: usize,
}

impl BufferPool {
    pub fn new(buffer_size: usize, max_buffers: usize) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(VecDeque::new())),
            buffer_size,
            max_buffers,
        }
    }
    
    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.pop_front().unwrap_or_else(|| vec![0u8; self.buffer_size])
    }
    
    pub fn return_buffer(&self, mut buffer: Vec<u8>) {
        let mut buffers = self.buffers.lock().unwrap();
        if buffers.len() < self.max_buffers {
            buffer.clear();
            buffer.resize(self.buffer_size, 0);
            buffers.push_back(buffer);
        }
    }
}
```

### Async Optimizations

```rust
use tokio::sync::Semaphore;

impl IrisClient {
    async fn process_packets_concurrently(&self, packets: Vec<PacketWrapper>) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(4)); // Limit concurrent processing
        let mut handles = Vec::new();
        
        for packet in packets {
            let permit = semaphore.clone().acquire_owned().await?;
            let fec_decoder = self.fec_decoder.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = permit; // Keep permit until task completes
                fec_decoder.process_packet(packet).await
            });
            
            handles.push(handle);
        }
        
        // Wait for all processing to complete
        for handle in handles {
            handle.await??;
        }
        
        Ok(())
    }
}
```

## Build and Deployment

### Cross-platform Compilation

```bash
# Build for current platform
cargo build --release

# Cross-compile for different targets
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-gnu

# Build with optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Docker Integration

```dockerfile
# Dockerfile for Linux client
FROM rust:1.75 as builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release --bin iris-linux-client

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/iris-linux-client /usr/local/bin/

ENTRYPOINT ["iris-linux-client"]
```

### Installation

```bash
# Install from source
git clone https://github.com/user/iris-project
cd iris-project/linux
cargo install --path .

# Run with systemd service
sudo cp iris-client.service /etc/systemd/system/
sudo systemctl enable iris-client
sudo systemctl start iris-client
```

## Testing and Validation

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fec_decoder() {
        let mut decoder = FECDecoder::new();
        
        // Create test packets
        let packets = create_test_packets(8, 4);
        
        // Process packets (missing some)
        for (i, packet) in packets.iter().enumerate() {
            if i < 6 { // Only send 6 out of 8 data packets
                let result = decoder.process_packet(packet.clone()).unwrap();
                assert!(result.is_none()); // Should not be able to reconstruct yet
            }
        }
        
        // Send one more packet to enable reconstruction
        let result = decoder.process_packet(packets[6].clone()).unwrap();
        assert!(result.is_some()); // Should now be able to reconstruct
    }
    
    #[test]
    fn test_certificate_validation() {
        let cert_manager = CertificateManager::new();
        let test_cert = load_test_certificate();
        
        assert!(cert_manager.validate_certificate(&test_cert).is_ok());
    }
}
```

### Integration Tests

```bash
# Network integration test
cargo test --test integration_test -- --nocapture

# Performance benchmark
cargo bench

# Run with test server
./test_server.sh &
cargo test --features integration_tests
```

## Documentation and Examples

### Usage Examples

```rust
// Example: Stream to file with custom settings
use iris_linux_client::*;

#[tokio::main]
async fn main() -> Result<()> {
    let config = ClientConfig {
        server_address: "https://localhost:50051".to_string(),
        stream_url: "rtmp://test.stream/live".to_string(),
        output_path: Some(PathBuf::from("output.mp4")),
        quality: "high".to_string(),
        ..Default::default()
    };
    
    let client = IrisClient::new(config).await?;
    client.start_streaming().await?;
    
    Ok(())
}
```

### API Documentation

Generate comprehensive API documentation:

```bash
cargo doc --open --no-deps
```

## Future Enhancements

### Planned Features

- **GUI Interface**: Desktop application with graphical interface
- **Hardware Acceleration**: GPU-accelerated video decoding
- **Plugin System**: Extensible plugin architecture for custom processing
- **Advanced Analytics**: Detailed quality and performance analytics

### Research Applications

This Linux client serves as a reference implementation for:

- **Cross-platform Compatibility**: Demonstrating protocol portability
- **Performance Benchmarking**: Baseline performance measurements
- **Algorithm Testing**: Validation of streaming algorithms
- **Integration Testing**: End-to-end system validation
