# IRIS - Secure Streaming with Fog Offloading

## Overview

IRIS is a comprehensive secure live streaming system with fog computing offloading capabilities, designed to deliver reliable video content over heterogeneous wireless networks. The system implements a novel architecture that combines WiFi Aware mesh networking, reinforcement learning-based adaptation, and certificate-based security protocols to provide robust streaming performance in challenging network conditions.

## System Architecture

The IRIS system employs a distributed microservices architecture with the following key components:

### Core Components

1. **Streaming Server (Rust)** - Central orchestrator for content distribution
2. **Android Client Application** - Mobile client with WiFi Aware mesh networking
3. **Encoder (Restreamer)** - Video ingestion and protocol-agnostic encoding
4. **Monitoring Stack** - Prometheus/Grafana for metrics and visualization
5. **Statistics Collection** - Real-time performance monitoring and analysis

### Network Architecture

```
[Encoder] -> [Streaming Server] -> [Android Clients]
                     |                    |
                     v                    v
            [Monitoring Stack]    [WiFi Aware Mesh]
```

## Technical Foundations

### Programming Languages & Frameworks
- **Rust**: Core streaming infrastructure (safe, concurrent, high-performance)
- **Java/Android**: Mobile client application with WiFi Aware integration
- **Protocol Buffers (gRPC)**: Inter-service communication
- **Docker**: Containerized deployment and development

### Network Protocols
- **WiFi Aware**: Device-to-device discovery and mesh networking
- **gRPC over HTTP/2**: Control plane communication
- **UDP**: High-performance data plane for video streaming
- **IPv6 Link-Local**: Mesh networking addressing

### Security Features
- **Certificate-based Authentication**: PKI infrastructure with server-signed certificates
- **Symmetric Key Management**: Session-based encryption with key rotation
- **Packet Signatures**: SHA-256 signed packets for integrity verification
- **Memory Safety**: Rust's ownership model prevents common vulnerabilities

## Key Features

### Adaptive Streaming
- **Reed-Solomon Error Correction**: Forward Error Correction (FEC) for packet recovery
- **Multi-path Transmission**: Simultaneous use of WiFi multicast, unicast, and mesh
- **Reinforcement Learning**: Q-learning for dynamic network adaptation
- **Real-time Quality Adjustment**: Adaptive bitrate and error correction parameters

### Mesh Networking
- **WiFi Aware Integration**: Android's Neighbor Awareness Networking (NAN)
- **Peer Discovery**: Automatic device discovery and mesh formation
- **Retransmission Coordination**: Collaborative packet recovery between peers
- **Loop Prevention**: TTL-based packet flooding control

### Monitoring & Analytics
- **Prometheus Metrics**: Time-series performance data collection
- **Grafana Dashboards**: Real-time visualization and alerting
- **Client Reporting**: Detailed statistics on packet delivery and network conditions
- **Performance Analysis**: Comprehensive testing and benchmarking tools

## Project Structure

```
├── android/              # Android client application
│   ├── app/             # Main Android app with WiFi Aware
│   └── wifi-aware-lib/  # WiFi Aware wrapper library
├── iris/                # Core streaming system
│   ├── client/          # Rust client implementation
│   ├── server/          # Streaming server
│   ├── shared-lib/      # Common utilities and protocols
│   └── proto/           # Protocol Buffer definitions
├── encoder/             # Video encoding service (Restreamer)
├── stats/               # Monitoring infrastructure
├── linux/               # Linux-specific client implementation
├── utils/               # Testing and development utilities
└── paper/               # Academic thesis documentation
```

## Research Contributions

This implementation supports research in:

1. **Fog Computing**: Edge-based content distribution and processing
2. **Wireless Mesh Networks**: WiFi Aware device-to-device communication
3. **Adaptive Streaming**: Machine learning-driven quality adaptation
4. **Network Security**: Certificate-based authentication in mobile environments
5. **Systems Programming**: Memory-safe high-performance networking in Rust

## Network Requirements

- **WiFi Aware Support**: Android 8.0+ devices with WiFi Aware capability
- **IPv6 Connectivity**: Link-local addressing for mesh networking
- **Multicast Support**: Network infrastructure supporting UDP multicast
- **Certificate Authority**: PKI infrastructure for device authentication

## Deployment Architecture

The system uses Docker containers orchestrated across a custom bridge network (`iris-network: 172.22.0.0/16`):

- **Encoder**: `172.22.0.101:8080` - Video ingestion and encoding
- **Streaming Server**: `172.22.0.102:50051` - Core streaming logic
- **Grafana**: `172.22.0.103:3000` - Monitoring dashboard
- **Prometheus**: `172.22.0.104:9090` - Metrics collection

## Development Setup

1. **Create Docker Network**:
   ```bash
    docker network create --driver=bridge --subnet=172.22.0.0/16 --opt com.docker.network.bridge.name=br_iris iris-network
   ```

2. **Build and Deploy**:
   ```bash
   # Start monitoring stack
   cd stats && docker-compose up -d
   
   # Start encoder
   cd encoder && docker-compose up -d
   
   # Start streaming infrastructure
   cd iris && docker-compose up -d
   ```

3. **Android Development**:
   - Import `android/` project in Android Studio
   - Build with NDK for Rust JNI integration
   - Deploy to WiFi Aware capable devices

## Performance Characteristics

- **Low Latency**: Sub-second streaming latency through optimized protocols
- **High Throughput**: Concurrent multicast and unicast delivery
- **Fault Tolerance**: Automatic failover and mesh-based recovery
- **Scalability**: Horizontal scaling through fog node distribution
- **Security**: End-to-end encryption with certificate validation

## Academic Context

This system implements concepts from:
- **IEEE 802.11s** mesh networking principles
- **DASH/HLS** adaptive streaming protocols
- **gRPC** microservices communication patterns  
- **Rust** systems programming best practices
- **Android WiFi Aware** proximity-based networking

## License & Usage

Developed as part of MSc thesis research at Universidade do Porto, Faculty of Sciences. See individual component READMEs for specific technical details and usage instructions.

## WiFi Aware Connection Timeout

The system implements a 60-second connection timeout for WiFi Aware sessions to balance power consumption with connection reliability.