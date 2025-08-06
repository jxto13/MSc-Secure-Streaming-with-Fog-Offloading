# Video Encoder Service (Restreamer)

## Overview

The encoder service provides video ingestion, transcoding, and protocol conversion capabilities for the IRIS streaming system. Built using the Datarhei Restreamer platform, this component handles heterogeneous video input sources and produces standardized output streams for distribution through the IRIS infrastructure.

## Architecture

### Service Design

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Input Source  │    │   Restreamer    │    │  IRIS Server    │
│                 │    │                 │    │                 │
│  • RTMP Stream  │───►│  • Transcoding  │───►│  • Distribution │
│  • HTTP Stream  │    │  • Protocol     │    │  • FEC Encoding │
│  • File Upload  │    │    Conversion   │    │  • Multicast    │
│  • Camera Feed  │    │  • Quality      │    │  • Security     │
└─────────────────┘    │    Adaptation   │    └─────────────────┘
                       └─────────────────┘
```

### Container Architecture

The encoder runs as a containerized service with the following characteristics:

- **Base Image**: `datarhei/restreamer:latest`
- **Network**: Connected to `iris-network` bridge (172.22.0.101)
- **Ports**: HTTP (8080), HTTPS (8181), RTMP (1935)
- **Privileges**: Elevated privileges for hardware acceleration access

## Technical Implementation

### Docker Configuration

```yaml
services:
    restreamer:
      container_name: restreamer
      image: datarhei/restreamer:latest
      volumes:
            - ./config:/core/config
            - ./data:/core/data
      ports:
          - 8080:8080    # HTTP API and Web Interface
          - 8181:8181    # HTTPS (if configured)
          - 1935:1935    # RTMP Ingestion
      privileged: true   # Hardware acceleration access
      networks: 
        iris-network:
            ipv4_address: 172.22.0.101
      restart: always
```

### Network Integration

#### Service Discovery
- **Static IP Assignment**: `172.22.0.101` for predictable service location
- **Bridge Network**: Connected to shared `iris-network` for inter-service communication
- **Port Exposure**: Standard streaming ports exposed for external access

#### Protocol Support
- **RTMP**: Real-Time Messaging Protocol for live stream ingestion
- **HTTP/HTTPS**: RESTful API and web-based management interface
- **WebRTC**: Browser-based real-time communication (future support)
- **HLS/DASH**: HTTP-based adaptive streaming output formats

### Configuration Management

#### Persistent Configuration
```
encoder/
├── config/
│   ├── config.json      # Main configuration
│   └── db.json         # Database state
└── data/
    ├── index.html      # Web interface
    ├── *.html         # Stream player pages
    └── channels/       # Stream channel data
```

#### Configuration Structure
```json
{
  "api": {
    "address": "0.0.0.0:8080",
    "enable": true,
    "cors": {
      "enable": true,
      "origins": ["*"]
    }
  },
  "rtmp": {
    "enable": true,
    "address": "0.0.0.0:1935"
  },
  "transcoding": {
    "hardware_acceleration": "auto",
    "default_codec": "h264",
    "quality_presets": ["low", "medium", "high"]
  }
}
```

## Input Source Management

### Supported Input Formats

#### Live Streaming Protocols
- **RTMP**: Primary ingestion protocol for live streams
  - OBS Studio integration
  - Hardware encoder support
  - Authentication and stream key management

- **RTSP**: IP camera and professional equipment integration
  - Network camera feeds
  - Surveillance system integration
  - Authentication support

- **HTTP/HTTPS**: Web-based stream ingestion
  - Browser-based streaming
  - API-driven stream creation
  - File upload and processing

#### File-based Sources
- **MP4/MOV**: Standard video file formats
- **AVI/MKV**: Legacy and container format support
- **WebM**: Web-optimized video format
- **Raw Streams**: Uncompressed video data

### Input Processing Pipeline

```
Input Source → Format Detection → Codec Analysis → Quality Assessment
      ↓                ↓                ↓              ↓
Stream Validation → Metadata Extraction → Quality Metrics → Transcoding Decision
```

#### Format Detection
- **Container Analysis**: Automatic detection of video container formats
- **Codec Identification**: Video and audio codec recognition
- **Stream Parameters**: Resolution, frame rate, bitrate analysis
- **Metadata Parsing**: Title, duration, quality information

#### Quality Assessment
- **Resolution Validation**: Supported resolution verification
- **Bitrate Analysis**: Input bitrate characterization
- **Frame Rate Detection**: Temporal quality assessment
- **Audio Quality**: Audio codec and bitrate evaluation

## Transcoding Engine

### Video Processing

#### Codec Support
```
Input Codecs          Output Codecs
├── H.264/AVC        ├── H.264/AVC (default)
├── H.265/HEVC       ├── H.265/HEVC (efficiency)
├── VP8/VP9          ├── VP9 (web optimized)
├── AV1              └── AV1 (future-proof)
└── MPEG-2/4
```

#### Quality Presets
- **Low Quality**: 480p, 1-2 Mbps, optimized for bandwidth-constrained environments
- **Medium Quality**: 720p, 3-5 Mbps, balanced quality and efficiency
- **High Quality**: 1080p, 6-10 Mbps, maximum quality for high-bandwidth scenarios
- **Custom**: User-defined parameters for specific requirements

#### Hardware Acceleration
```
Acceleration Type     Supported Operations
├── NVIDIA NVENC     ├── H.264/H.265 encoding
├── Intel Quick Sync ├── Hardware-accelerated encoding
├── AMD VCE          ├── GPU-based transcoding
└── Software (CPU)   └── Fallback for compatibility
```

### Audio Processing

#### Audio Codec Support
- **AAC**: Primary audio codec for web delivery
- **MP3**: Legacy compatibility and broad support
- **Opus**: High-efficiency audio codec for low-latency applications
- **PCM**: Uncompressed audio for professional applications

#### Audio Quality Adaptation
- **Bitrate Adaptation**: Dynamic audio bitrate adjustment
- **Sample Rate Conversion**: Format-appropriate sample rate selection
- **Channel Configuration**: Mono/stereo/surround sound optimization
- **Normalization**: Audio level standardization

## Output Stream Generation

### Protocol Conversion

#### Adaptive Streaming Formats
- **HLS (HTTP Live Streaming)**: Apple-developed protocol for broad compatibility
- **DASH (Dynamic Adaptive Streaming)**: MPEG standard for adaptive delivery
- **Smooth Streaming**: Microsoft protocol for Windows ecosystem
- **Custom Segmentation**: IRIS-specific segmentation for optimal FEC

#### Stream Segmentation
```
Continuous Stream → Time-based Segments → Quality Variants → Manifest Generation
       ↓                    ↓                    ↓              ↓
    Raw Video        4-10 second chunks    Multiple bitrates   .m3u8/.mpd files
```

#### Manifest Generation
- **HLS Manifests**: .m3u8 playlist files with segment references
- **DASH Manifests**: .mpd files with adaptation set definitions
- **Quality Ladders**: Multiple resolution/bitrate combinations
- **Dynamic Updates**: Real-time manifest updates for live streams

### Integration with IRIS Server

#### Stream Delivery
- **HTTP Streaming**: Direct HTTP-based stream delivery to IRIS server
- **Network Optimization**: Localhost delivery for minimal latency
- **Format Standardization**: Consistent output format for IRIS processing
- **Quality Negotiation**: Dynamic quality adjustment based on network conditions

#### Metadata Propagation
```json
{
  "stream_id": "unique_stream_identifier",
  "format": "h264_aac",
  "resolution": "1920x1080",
  "framerate": 30,
  "bitrate": 5000000,
  "segments": [
    {
      "index": 0,
      "duration": 6.0,
      "url": "/segments/segment_000.ts"
    }
  ]
}
```

## Management Interface

### Web-based Administration

#### Dashboard Features
- **Live Stream Monitoring**: Real-time stream status and metrics
- **Source Configuration**: Input source management and setup
- **Quality Control**: Transcoding parameter adjustment
- **Performance Metrics**: CPU, memory, and network utilization

#### Stream Management
- **Channel Creation**: New stream channel setup and configuration
- **Source Switching**: Dynamic input source changes
- **Recording Control**: Stream recording and archival
- **Publishing Control**: Stream start/stop and scheduling

### API Integration

#### RESTful API
```http
GET /api/v1/streams                  # List active streams
POST /api/v1/streams                 # Create new stream
PUT /api/v1/streams/{id}             # Update stream configuration
DELETE /api/v1/streams/{id}          # Stop and remove stream
GET /api/v1/streams/{id}/stats       # Stream performance metrics
```

#### WebSocket API
- **Real-time Updates**: Live stream status notifications
- **Performance Metrics**: Continuous monitoring data
- **Event Notifications**: Stream start/stop/error events
- **Control Commands**: Real-time stream control

## Performance Optimization

### Hardware Utilization

#### CPU Optimization
- **Multi-threading**: Parallel processing for multiple streams
- **SIMD Instructions**: Vectorized operations for video processing
- **Cache Optimization**: Efficient memory access patterns
- **Process Affinity**: CPU core binding for consistent performance

#### GPU Acceleration
- **NVENC/QuickSync**: Hardware-accelerated encoding
- **Memory Management**: Efficient GPU memory utilization
- **Pipeline Optimization**: Overlapped processing stages
- **Quality Tuning**: Hardware-specific quality optimization

#### Memory Management
- **Buffer Pools**: Pre-allocated memory buffers
- **Zero-Copy Operations**: Direct memory access where possible
- **Garbage Collection**: Efficient memory cleanup
- **Memory Mapping**: File-based memory management for large streams

### Network Optimization

#### Bandwidth Management
- **Quality Adaptation**: Dynamic bitrate adjustment
- **Buffer Management**: Optimal buffering strategies
- **Congestion Control**: Network-aware quality adjustment
- **Priority Queuing**: Critical packet prioritization

#### Latency Reduction
- **Low-Latency Encoding**: Minimal encoding delay
- **Pipeline Parallelism**: Overlapped encoding stages
- **Network Buffering**: Minimal network buffer sizes
- **Real-time Processing**: Frame-by-frame processing optimization

## Monitoring and Analytics

### Performance Metrics

#### Stream Quality Metrics
```json
{
  "stream_id": "stream_001",
  "metrics": {
    "input_fps": 29.97,
    "output_fps": 29.97,
    "dropped_frames": 0,
    "encoding_delay": 45,
    "bitrate_actual": 4987432,
    "bitrate_target": 5000000
  }
}
```

#### System Resource Metrics
- **CPU Utilization**: Per-core and overall CPU usage
- **Memory Usage**: RAM and GPU memory consumption
- **Network Bandwidth**: Input/output bandwidth utilization
- **Disk I/O**: Read/write operations for storage-based streams

#### Quality Assessment
- **PSNR/SSIM**: Objective video quality measurements
- **Bitrate Efficiency**: Rate-distortion optimization
- **Latency Tracking**: End-to-end delay measurement
- **Error Rates**: Stream interruption and error frequency

### Logging and Diagnostics

#### Comprehensive Logging
- **Stream Events**: Start, stop, configuration changes
- **Error Tracking**: Detailed error messages and stack traces
- **Performance Logs**: Resource utilization over time
- **Network Events**: Connection status and bandwidth changes

#### Debug Information
- **Encoder Parameters**: Detailed encoding configuration
- **Stream Analysis**: Frame-by-frame analysis capabilities
- **Network Diagnostics**: Connection quality and issues
- **Hardware Status**: Acceleration hardware availability

## Configuration Examples

### Basic RTMP Stream Setup
```json
{
  "input": {
    "type": "rtmp",
    "url": "rtmp://localhost:1935/live/stream_key",
    "authentication": {
      "required": true,
      "key": "secure_stream_key"
    }
  },
  "output": {
    "format": "hls",
    "quality": "medium",
    "segments": {
      "duration": 6,
      "count": 5
    }
  }
}
```

### Multi-Quality Output
```json
{
  "input": {
    "type": "file",
    "path": "/data/input/source.mp4"
  },
  "outputs": [
    {
      "quality": "high",
      "resolution": "1920x1080",
      "bitrate": 8000000,
      "codec": "h264"
    },
    {
      "quality": "medium",
      "resolution": "1280x720", 
      "bitrate": 4000000,
      "codec": "h264"
    },
    {
      "quality": "low",
      "resolution": "854x480",
      "bitrate": 2000000,
      "codec": "h264"
    }
  ]
}
```

## Integration Patterns

### IRIS Server Integration

#### Stream Discovery
- **Service Registration**: Automatic stream availability notification
- **Endpoint Discovery**: Dynamic endpoint resolution
- **Health Monitoring**: Continuous service health verification
- **Load Balancing**: Multiple encoder instance support

#### Quality Coordination
- **Adaptive Requests**: Quality adjustment requests from IRIS server
- **Network Feedback**: Real-time network condition reporting
- **Quality Negotiation**: Dynamic quality parameter adjustment
- **Emergency Fallback**: Quality reduction under extreme conditions

### Monitoring Integration

#### Prometheus Metrics
```
# Encoder performance metrics
encoder_streams_active{instance="encoder"} 3
encoder_cpu_usage{instance="encoder"} 75.2
encoder_memory_usage{instance="encoder"} 2048
encoder_bitrate_output{stream="stream_001"} 4987432
```

#### Grafana Dashboards
- **Stream Overview**: Active streams and their status
- **Performance Monitoring**: Resource utilization trends
- **Quality Metrics**: Video quality and encoding efficiency
- **Alert Configuration**: Threshold-based alerting

## Security Considerations

### Input Validation
- **Stream Authentication**: Secure stream key validation
- **Content Filtering**: Malicious content detection
- **Rate Limiting**: Protection against DoS attacks
- **Input Sanitization**: Safe handling of user-provided data

### Output Security
- **Stream Encryption**: Optional stream encryption for sensitive content
- **Access Control**: Authentication for stream access
- **Audit Logging**: Comprehensive access and modification logging
- **Secure Transmission**: HTTPS/TLS for all control communications

## Deployment and Scaling

### Container Orchestration
```yaml
# Kubernetes deployment example
apiVersion: apps/v1
kind: Deployment
metadata:
  name: restreamer-encoder
spec:
  replicas: 3
  selector:
    matchLabels:
      app: restreamer
  template:
    spec:
      containers:
      - name: restreamer
        image: datarhei/restreamer:latest
        resources:
          requests:
            cpu: 500m
            memory: 1Gi
          limits:
            cpu: 2
            memory: 4Gi
```

### Load Balancing
- **Stream Distribution**: Multiple encoder instances for load distribution
- **Health Checking**: Automatic unhealthy instance removal
- **Session Affinity**: Consistent stream routing for multi-part uploads
- **Geographic Distribution**: Encoder placement for optimal latency

## Future Enhancements

### Planned Features
- **AI-based Quality**: Machine learning for optimal encoding parameters
- **Edge Processing**: Distributed encoding across edge nodes
- **Advanced Codecs**: AV1 and next-generation codec support
- **WebRTC Integration**: Browser-based live streaming capabilities

### Research Applications
- **Adaptive Encoding**: Network-aware encoding optimization
- **Quality Prediction**: ML-based quality forecasting
- **Distributed Processing**: Collaborative encoding across multiple nodes
- **Energy Efficiency**: Power-aware encoding optimization
