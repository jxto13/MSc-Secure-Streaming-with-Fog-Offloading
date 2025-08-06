# IRIS Streaming Infrastructure

## Overview

The IRIS streaming infrastructure represents the core distributed system for secure video streaming with fog offloading capabilities. Built entirely in Rust, this component provides the foundational streaming server, client implementation, and shared libraries that enable high-performance, secure content delivery across heterogeneous wireless networks.

## Architecture

### System Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  IRIS Server    │    │  IRIS Client    │    │  Shared Lib     │
│                 │    │                 │    │                 │
│  • gRPC API     │◄──►│  • Stream Recv  │    │  • Proto Defs   │
│  • Multicast    │    │  • Mesh Logic   │    │  • Crypto Utils │
│  • FEC Encoding │    │  • FEC Decoding │    │  • Common Types │
│  • Security     │    │  • Reporting    │    │  • Network Lib  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │  Protocol Defs  │
                    │                 │
                    │  • streaming    │
                    │  • edge_client  │
                    │  • certificates │
                    └─────────────────┘
```

## IRIS Server

### Core Functionality

The IRIS server serves as the central orchestrator for video distribution, implementing:

#### Multi-Protocol Streaming
- **WiFi Multicast**: Primary distribution mechanism for efficient bandwidth utilization
- **WiFi Unicast**: Fallback mechanism for devices unable to receive multicast
- **4G Unicast**: Future support for cellular backup connections
- **Mesh Coordination**: Integration with client mesh networks for collaborative distribution

#### Forward Error Correction (FEC)
```rust
// Reed-Solomon error correction implementation
use reed_solomon_16::*;

let encoder = Encoder::new(data_shards, parity_shards);
let encoded_data = encoder.encode(&original_data)?;
```

- **Reed-Solomon Encoding**: Configurable error correction ratios
- **Matrix-based Segmentation**: Video frames segmented into correctable matrices
- **Adaptive Redundancy**: Dynamic adjustment based on network conditions
- **Recovery Optimization**: Minimal overhead for error-free transmission

#### Certificate-based Security
```rust
// Certificate signing and validation
pub async fn sign_certificate(
    &self,
    request: CertificateRequest
) -> Result<CertificateResponse> {
    let csr = parse_csr(&request.csr)?;
    let signed_cert = self.ca.sign_certificate(csr).await?;
    Ok(CertificateResponse { signed_cert })
}
```

- **PKI Infrastructure**: Complete certificate authority implementation
- **Client Authentication**: Device identity verification through certificates
- **Key Management**: Symmetric key distribution with periodic rotation
- **Signature Verification**: SHA-256 packet signatures for integrity

### Network Scheduler Pipeline

The server implements a sophisticated scheduling pipeline:

```rust
pub struct NetworkScheduler {
    multicast_scheduler: MulticastScheduler,
    unicast_scheduler: UnicastScheduler,
    mesh_coordinator: MeshCoordinator,
    prediction_service: PredictionService,
}
```

#### Multicast Scheduler (Primary)
- **Group Management**: Dynamic multicast group membership
- **Rate Control**: Adaptive bitrate based on weakest link
- **Packet Prioritization**: Critical frame prioritization for quality maintenance
- **Congestion Avoidance**: Proactive rate reduction on network stress

#### Unicast Scheduler (Fallback)
- **Per-client Adaptation**: Individual client quality adjustment
- **Load Balancing**: Request distribution across available bandwidth
- **Session Management**: Stateful client connection tracking
- **Quality Negotiation**: Dynamic quality parameter adjustment

#### Mesh Coordination
- **Retransmission Orchestration**: Coordinated packet recovery across mesh
- **Topology Awareness**: Understanding of mesh network structure
- **Load Distribution**: Optimal distribution of recovery responsibilities
- **Conflict Resolution**: Prevention of duplicate recovery attempts

### Machine Learning Integration

#### Reinforcement Learning Adaptation
```rust
// Q-learning implementation for network adaptation
pub struct QLearningAgent {
    q_table: HashMap<State, HashMap<Action, f64>>,
    learning_rate: f64,
    discount_factor: f64,
    exploration_rate: f64,
}
```

- **State Space**: Network conditions, client capabilities, quality metrics
- **Action Space**: Bitrate adjustments, FEC ratios, transmission modes
- **Reward Function**: Quality-of-experience optimization with efficiency constraints
- **Exploration Strategy**: Epsilon-greedy with adaptive exploration decay

#### Client Session Management
- **Performance Tracking**: Per-client metrics collection and analysis
- **Predictive Modeling**: Quality prediction based on historical performance
- **Anomaly Detection**: Identification of network issues and device problems
- **Adaptive Policies**: Custom adaptation strategies per client class

### Service Architecture

#### gRPC Services
```protobuf
service CertificateService {
    rpc SignCertificate(CertificateRequest) returns (CertificateResponse);
    rpc VerifyCertificate(SignedCertificateRequest) returns (ValidationResponse);
}

service KeyService {
    rpc RequestCurrentKey(RequestKey) returns (KeyResponse);
}

service ReportService {
    rpc ReportSink(ClientReport) returns (TimestampResponse);
}

service NtsService {
    rpc GetCurrentTimestamp(TimestampRequest) returns (TimestampResponse);
}
```

#### Certificate Service
- **CSR Processing**: Certificate Signing Request validation and processing
- **Certificate Validation**: Real-time certificate verification for clients
- **Revocation Management**: Certificate revocation list maintenance
- **Authority Management**: Root CA and intermediate certificate handling

#### Key Service
- **Session Key Distribution**: Encrypted symmetric key delivery
- **Key Rotation**: Automatic key updates with seamless transition
- **Epoch Management**: Time-based key rotation for forward secrecy
- **Piggyback Data**: Configuration updates bundled with key distribution

#### Report Service
- **Telemetry Collection**: Comprehensive client performance metrics
- **Matrix Reporting**: Bitmap-based packet reception status
- **Aggregation**: Real-time metrics aggregation and analysis
- **Storage**: Persistent metrics storage for long-term analysis

#### Network Time Service
- **Timestamp Synchronization**: Coordinated Universal Time distribution
- **Latency Compensation**: Network delay compensation for accurate timing
- **Clock Drift Correction**: Automatic adjustment for client clock variations
- **Precision Timing**: Sub-millisecond accuracy for stream synchronization

### Performance Optimizations

#### Async Runtime Management
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = StreamingServer::new().await?;
    server.run().await?;
    Ok(())
}
```

- **Tokio Runtime**: Multi-threaded async execution for high concurrency
- **Connection Pooling**: Efficient client connection management
- **Resource Sharing**: Zero-copy data structures where possible
- **Memory Optimization**: Careful memory allocation patterns for low latency

#### Network Optimizations
- **Socket Tuning**: TCP and UDP socket buffer optimization
- **Kernel Bypass**: User-space networking where beneficial
- **SIMD Operations**: Vectorized processing for data encoding
- **CPU Affinity**: Thread-to-core binding for consistent performance

## IRIS Client

### Streaming Client Implementation

The Rust client provides high-performance video reception and processing:

#### Stream Processing Pipeline
```rust
pub struct StreamProcessor {
    decoder: FECDecoder,
    buffer_manager: PacketBufferManager,
    mesh_interface: MeshInterface,
    reporting: ReportingService,
}
```

#### Packet Reception
- **Multi-source Aggregation**: Simultaneous reception from server and mesh peers
- **Duplicate Detection**: Efficient identification and elimination of duplicate packets
- **Ordering**: Sequence number-based packet ordering and buffering
- **Timeout Management**: Automatic packet loss detection and recovery initiation

#### FEC Decoding
```rust
// Reed-Solomon decoding with error recovery
let decoder = Decoder::new(data_shards, parity_shards);
match decoder.reconstruct(&received_packets) {
    Ok(recovered_data) => process_video_frame(recovered_data),
    Err(_) => request_retransmission(missing_packets),
}
```

- **Matrix Reconstruction**: Recovery of missing packets from available data
- **Partial Recovery**: Processing of partially received matrices
- **Quality Adaptation**: Graceful degradation when insufficient data available
- **Recovery Optimization**: Intelligent selection of recovery strategies

#### Mesh Interface
- **Peer Discovery**: Integration with WiFi Aware mesh networking
- **Request Broadcasting**: Efficient retransmission request distribution
- **Response Handling**: Processing of peer-provided packet data
- **Load Balancing**: Distribution of requests across available peers

### Client Reporting

#### Metrics Collection
```rust
pub struct ClientMetrics {
    multicast_retransmissions: u32,
    unicast_retransmissions: u32,
    wifi_aware_retransmissions: u32,
    packet_loss_rate: f32,
    quality_metrics: QualityMetrics,
}
```

- **Reception Statistics**: Detailed packet reception and loss tracking
- **Quality Metrics**: Real-time quality assessment and reporting
- **Network Performance**: Bandwidth utilization and latency measurements
- **Error Tracking**: Comprehensive error classification and reporting

#### Security Integration
- **Report Signing**: Cryptographic signatures for report authenticity
- **Counter-based Protection**: Replay attack prevention through sequence counters
- **Compression**: Efficient data compression for reduced network overhead
- **Batch Reporting**: Aggregated reporting to minimize network impact

## Shared Library

### Common Utilities

The shared library provides essential components used across client and server:

#### Protocol Definitions
```rust
// Generated from Protocol Buffer definitions
pub mod streaming_server {
    include!(concat!(env!("OUT_DIR"), "/streaming_server.rs"));
}

pub mod edge_client {
    include!(concat!(env!("OUT_DIR"), "/edge_client.rs"));
}
```

#### Cryptographic Utilities
- **Hash Functions**: SHA-256 implementation for signatures and verification
- **Symmetric Encryption**: AES encryption for payload protection
- **Key Derivation**: PBKDF2 and HKDF for secure key generation
- **Certificate Handling**: X.509 certificate parsing and validation

#### Network Abstractions
- **Socket Management**: Cross-platform socket creation and configuration
- **Address Resolution**: IPv4/IPv6 address handling and resolution
- **Protocol Serialization**: Efficient binary protocol encoding/decoding
- **Error Handling**: Comprehensive error type definitions and handling

#### Data Structures
```rust
// Matrix representation for FEC processing
pub struct Matrix<T> {
    data: Vec<T>,
    rows: usize,
    cols: usize,
}

// Packet buffer management
pub struct PacketBuffer {
    packets: HashMap<u32, Vec<u8>>,
    sequence_numbers: BTreeSet<u32>,
    last_received: u32,
}
```

## Protocol Buffer Integration

### Build System Integration
```rust
// build.rs - Protocol compilation
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/streaming_server.proto"], &["proto/"])?;
    Ok(())
}
```

### Message Definitions

#### Streaming Protocol
- **TransmissionPacket**: Core video data with metadata
- **RetransmissionPacket**: Recovery packet format
- **XORPacket**: Optimized recovery using XOR operations
- **PacketWrapper**: Unified packet type for different transmission modes

#### Security Protocol
- **CertificateRequest/Response**: PKI certificate management
- **KeyRequest/Response**: Symmetric key distribution
- **ClientReport**: Performance and security reporting

#### Mesh Protocol
- **MeshPacket**: Peer-to-peer communication format
- **HeartbeatPacket**: Mesh topology maintenance
- **RetransmissionRequest/Response**: Collaborative packet recovery

## Development and Deployment

### Build Configuration
```toml
[workspace]
members = [
    "iris/client",
    "iris/server", 
    "iris/shared-lib"
]

[dependencies]
tokio = { version = "1.45.0", features = ["rt-multi-thread"] }
tonic = { version = "0.13.1", features = ["transport", "tls-ring"] }
prost = "0.13.1"
reed-solomon-16 = "0.1.0"
```

### Docker Integration
```dockerfile
# Multi-stage build for optimized containers
FROM rust:1.75 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /usr/src/app/target/release/streaming_server /usr/local/bin/
CMD ["streaming_server"]
```

### Configuration Management
```toml
# Server configuration
[server]
bind_address = "0.0.0.0:50051"
certificate_path = "/etc/iris/server.crt"
private_key_path = "/etc/iris/server.key"

[multicast]
address = "224.0.0.251"
port = 8080
ttl = 64

[fec]
data_shards = 8
parity_shards = 4
matrix_size = 64
```

## Performance Characteristics

### Throughput Optimization
- **Zero-Copy Networking**: Direct buffer manipulation without copying
- **Vectorized Operations**: SIMD instructions for data processing
- **Lock-Free Data Structures**: Atomic operations for shared state
- **Memory Pool Management**: Pre-allocated buffers for consistent performance

### Latency Minimization
- **Interrupt-driven I/O**: Immediate response to network events
- **Priority Queuing**: Critical packet prioritization
- **Predictive Prefetching**: Anticipatory data loading
- **Hardware Acceleration**: Leveraging available crypto and network acceleration

### Scalability Features
- **Horizontal Scaling**: Multi-instance deployment support
- **Load Balancing**: Request distribution across server instances
- **State Management**: Stateless design where possible for easy scaling
- **Resource Monitoring**: Real-time resource utilization tracking

## Testing and Quality Assurance

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fec_encoding_decoding() {
        let original_data = vec![1, 2, 3, 4, 5];
        let encoded = encode_with_fec(&original_data, 2, 1).await?;
        let decoded = decode_with_fec(&encoded, 2, 1).await?;
        assert_eq!(original_data, decoded);
    }
}
```

### Integration Testing
- **Network Simulation**: Controlled network condition testing
- **Load Testing**: High-concurrency client simulation
- **Fault Injection**: Error condition and recovery testing
- **Security Testing**: Cryptographic implementation verification

### Performance Benchmarking
- **Throughput Measurement**: Maximum data rate characterization
- **Latency Profiling**: End-to-end latency analysis
- **Resource Utilization**: CPU and memory usage optimization
- **Scalability Testing**: Multi-client performance evaluation

## Security Considerations

### Threat Model
- **Man-in-the-Middle**: Certificate validation prevents MITM attacks
- **Replay Attacks**: Counter-based protection in all protocols
- **Data Integrity**: SHA-256 signatures verify packet authenticity
- **Eavesdropping**: AES encryption protects data confidentiality

### Security Implementation
- **Rust Safety**: Memory safety prevents buffer overflow vulnerabilities
- **Crypto Libraries**: Vetted cryptographic library usage
- **Input Validation**: Comprehensive input sanitization
- **Error Handling**: Secure error handling without information leakage

## Future Development

### Planned Enhancements
- **WebRTC Integration**: Browser-based client support
- **Hardware Acceleration**: GPU-based FEC processing
- **Advanced ML**: Deep learning for quality optimization
- **Edge Computing**: Distributed processing capabilities

### Research Applications
- **Adaptive Algorithms**: Network-aware streaming optimization
- **Security Protocols**: Novel authentication mechanisms
- **Performance Analysis**: Large-scale deployment studies
- **Protocol Innovation**: Next-generation streaming protocols
