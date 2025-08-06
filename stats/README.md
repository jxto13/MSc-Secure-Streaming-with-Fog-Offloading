# Monitoring and Analytics Infrastructure

## Overview

The monitoring and analytics infrastructure provides comprehensive real-time observability for the IRIS streaming system. Built on the industry-standard Prometheus and Grafana stack, this component enables detailed performance monitoring, alerting, and system optimization through data-driven insights.

## Architecture

### Service Topology

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   IRIS Server   │    │   Prometheus    │    │    Grafana      │
│                 │    │                 │    │                 │
│  • Metrics API  │───►│  • Data Collection│◄──│  • Dashboards   │
│  • Performance  │    │  • Time Series  │    │  • Alerting     │
│  • Client Data  │    │  • Aggregation  │    │  • Visualization│
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         v                       v                       v
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Android Clients│    │  Encoder Service│    │  External APIs  │
│                 │    │                 │    │                 │
│  • Client Metrics│   │  • Stream Stats │    │  • Notifications│
│  • Network Data │    │  • Encoder Perf │    │  • Integrations │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Container Architecture

The monitoring stack operates as interconnected containerized services:

- **Prometheus**: `172.22.0.104:9090` - Metrics collection and storage
- **Grafana**: `172.22.0.103:3000` - Visualization and alerting interface
- **Network**: Shared `iris-network` bridge for service communication

## Prometheus Configuration

### Service Discovery and Scraping

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "first_rules.yml"
  - "second_rules.yml"

scrape_configs:
  - job_name: 'iris-server'
    static_configs:
      - targets: ['172.22.0.102:50051']
    metrics_path: '/metrics'
    scrape_interval: 5s
    
  - job_name: 'encoder'
    static_configs:
      - targets: ['172.22.0.101:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
```

### Metrics Collection Strategy

#### Pull-based Architecture
- **Target Discovery**: Automatic service endpoint discovery
- **Health Monitoring**: Continuous target health verification
- **Retry Logic**: Intelligent retry mechanisms for failed scrapes
- **Load Distribution**: Distributed scraping for high-scale deployments

#### Time Series Database
```
Metric Name: iris_streaming_packets_sent_total
Labels: {instance="172.22.0.102:50051", job="iris-server", stream_id="stream_001"}
Value: 15847
Timestamp: 1640995200
```

- **High Cardinality Support**: Efficient storage of labeled time series
- **Compression**: Advanced compression algorithms for storage efficiency
- **Retention Policies**: Configurable data retention and downsampling
- **Query Performance**: Optimized query engine for real-time analytics

## Grafana Visualization

### Dashboard Architecture

#### System Overview Dashboard
```json
{
  "dashboard": {
    "id": null,
    "title": "IRIS System Overview",
    "panels": [
      {
        "title": "Active Streams",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(iris_streams_active)",
            "legendFormat": "Active Streams"
          }
        ]
      },
      {
        "title": "Network Throughput",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(iris_bytes_transmitted_total[5m])",
            "legendFormat": "{{instance}}"
          }
        ]
      }
    ]
  }
}
```

#### Performance Monitoring Panels
- **Stream Quality Metrics**: Real-time video quality indicators
- **Network Performance**: Bandwidth utilization and latency tracking
- **System Resources**: CPU, memory, and disk utilization
- **Error Rates**: Stream failures and recovery statistics

#### Client-side Analytics
- **Mesh Network Topology**: WiFi Aware peer connections and topology
- **Packet Loss Analysis**: Detailed packet loss patterns and recovery
- **Quality of Experience**: User-perceived quality metrics
- **Device Performance**: Individual client performance characteristics

### Alert Management

#### Alert Rules Configuration
```yaml
# alerting_rules.yml
groups:
  - name: iris_streaming_alerts
    rules:
      - alert: HighPacketLoss
        expr: rate(iris_packets_lost_total[5m]) > 0.05
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High packet loss detected"
          description: "Packet loss rate is {{ $value }} for stream {{ $labels.stream_id }}"
          
      - alert: StreamingServerDown
        expr: up{job="iris-server"} == 0
        for: 30s
        labels:
          severity: critical
        annotations:
          summary: "IRIS streaming server is down"
```

#### Notification Channels
- **Email Notifications**: SMTP-based alert delivery
- **Slack Integration**: Real-time team notifications
- **Webhook Support**: Custom notification endpoints
- **PagerDuty Integration**: Incident management system integration

## Metrics Collection Framework

### Server-side Metrics

#### Streaming Performance
```rust
// Prometheus metrics integration in Rust
use prometheus::{Histogram, Counter, Gauge, register_histogram, register_counter};

// Packet transmission metrics
lazy_static! {
    static ref PACKETS_SENT: Counter = register_counter!(
        "iris_packets_sent_total",
        "Total number of packets sent"
    ).unwrap();
    
    static ref PACKET_LATENCY: Histogram = register_histogram!(
        "iris_packet_latency_seconds",
        "Packet transmission latency"
    ).unwrap();
}

// Usage in streaming code
PACKETS_SENT.inc();
let timer = PACKET_LATENCY.start_timer();
send_packet(&packet).await;
timer.observe_duration();
```

#### Quality Metrics
- **Bitrate Adaptation**: Real-time bitrate adjustment tracking
- **FEC Performance**: Error correction efficiency and utilization
- **Certificate Operations**: PKI operations performance and success rates
- **Key Rotation**: Security key update frequency and performance

#### Network Metrics
- **Connection Statistics**: Active connections and session duration
- **Bandwidth Utilization**: Network capacity usage and efficiency
- **Multicast Performance**: Group membership and delivery success
- **Mesh Coordination**: Inter-client mesh networking statistics

### Client-side Metrics

#### Reception Quality
```java
// Android client metrics collection
public class MetricsCollector {
    private final Counter packetsReceived = Counter.build()
        .name("iris_client_packets_received_total")
        .help("Total packets received by client")
        .register();
        
    private final Histogram bufferLevel = Histogram.build()
        .name("iris_client_buffer_level_seconds")
        .help("Current buffer level in seconds")
        .register();
}
```

#### WiFi Aware Metrics
- **Peer Discovery**: Device discovery success rates and timing
- **Connection Establishment**: WiFi Aware connection setup performance
- **Mesh Topology**: Network topology changes and stability
- **Retransmission Efficiency**: Peer-to-peer packet recovery performance

#### Device Performance
- **CPU Utilization**: Client-side processing load
- **Memory Usage**: Application memory consumption patterns
- **Battery Impact**: Power consumption attribution
- **Network Interface Statistics**: WiFi, cellular, and mesh interface utilization

### Encoder Metrics

#### Transcoding Performance
```
# Encoder metrics
encoder_streams_active{instance="encoder"} 3
encoder_fps{stream="stream_001"} 29.97
encoder_bitrate_input{stream="stream_001"} 8000000
encoder_bitrate_output{stream="stream_001"} 5000000
encoder_cpu_usage_percent{instance="encoder"} 75.2
encoder_memory_usage_bytes{instance="encoder"} 2147483648
```

#### Quality Assessment
- **Rate-Distortion Optimization**: Encoding efficiency metrics
- **Hardware Acceleration**: GPU utilization and performance gains
- **Latency Tracking**: Encoding delay and real-time performance
- **Error Rates**: Encoding failures and recovery statistics

## Data Storage and Retention

### Time Series Storage

#### Storage Architecture
```
Raw Metrics (15s resolution) → 7 days retention
5m Aggregated Data → 30 days retention  
1h Aggregated Data → 1 year retention
1d Aggregated Data → 5 years retention
```

#### Downsampling Rules
```yaml
# Recording rules for downsampling
groups:
  - name: iris_recording_rules
    interval: 30s
    rules:
      - record: iris:packet_rate_5m
        expr: rate(iris_packets_sent_total[5m])
        
      - record: iris:bandwidth_utilization_5m
        expr: rate(iris_bytes_transmitted_total[5m]) * 8
```

#### Storage Optimization
- **Compression**: Advanced time series compression algorithms
- **Sharding**: Distributed storage across multiple nodes
- **Backup Strategy**: Automated backup and disaster recovery
- **Query Optimization**: Index optimization for fast query performance

### Long-term Analytics

#### Historical Analysis
- **Trend Identification**: Long-term performance trend analysis
- **Capacity Planning**: Resource requirement forecasting
- **Quality Evolution**: Stream quality improvement tracking
- **Usage Patterns**: User behavior and system utilization patterns

#### Data Export and Integration
```python
# Data export example
import prometheus_api_client

prom = PrometheusConnect(url="http://172.22.0.104:9090")
metric_data = prom.get_metric_range_data(
    metric_name="iris_packet_rate_5m",
    start_time=datetime.now() - timedelta(days=7),
    end_time=datetime.now()
)
```

## Custom Dashboards

### Network Performance Dashboard

#### Real-time Network Visualization
```
┌─────────────────────────────────────────────────────────────┐
│  Network Throughput (Mbps)                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │     ∩∩∩                                              │   │
│  │    ∩   ∩∩∩                                           │   │
│  │  ∩∩       ∩∩∩                                        │   │
│  │ ∩           ∩∩∩                                      │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Packet Loss Rate (%)          Active Connections          │
│  ┌─────────────────┐           ┌─────────────────┐         │
│  │     0.12%       │           │       247       │         │
│  └─────────────────┘           └─────────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

#### Quality of Service Metrics
- **Latency Distribution**: End-to-end latency histograms
- **Jitter Analysis**: Network stability and variation metrics
- **Throughput Efficiency**: Actual vs. theoretical bandwidth utilization
- **Recovery Performance**: Error correction and retransmission effectiveness

### System Health Dashboard

#### Infrastructure Monitoring
```
System Resource Utilization
├── CPU Usage: [████████░░] 80%
├── Memory: [██████░░░░] 60%
├── Disk I/O: [███░░░░░░░] 30%
└── Network: [████████░░] 85%

Service Status
├── IRIS Server: ✓ Healthy
├── Encoder: ✓ Healthy  
├── Prometheus: ✓ Healthy
└── Grafana: ✓ Healthy
```

#### Alert Status Overview
- **Active Alerts**: Current system alerts and their severity
- **Alert History**: Recent alert patterns and resolution times
- **Escalation Status**: Alert escalation and notification delivery
- **Performance Impact**: Resource impact of monitoring infrastructure

### Client Analytics Dashboard

#### Mobile Client Performance
```json
{
  "panels": [
    {
      "title": "WiFi Aware Connections",
      "type": "graph",
      "metrics": [
        "iris_wifi_aware_peers_connected",
        "iris_wifi_aware_discovery_attempts",
        "iris_wifi_aware_connection_failures"
      ]
    },
    {
      "title": "Mesh Network Topology",
      "type": "node-graph",
      "data_source": "iris_mesh_topology"
    }
  ]
}
```

#### User Experience Metrics
- **Quality of Experience (QoE)**: Subjective quality assessment
- **Session Duration**: User engagement and retention metrics
- **Error Recovery**: User-perceived error frequency and recovery time
- **Device Diversity**: Performance across different device types and capabilities

## Alerting and Automation

### Alert Severity Levels

#### Critical Alerts
- **Service Unavailability**: Core service failures requiring immediate attention
- **Security Incidents**: Authentication failures or potential security breaches
- **Data Loss**: Streaming interruptions affecting user experience
- **System Overload**: Resource exhaustion requiring immediate intervention

#### Warning Alerts
- **Performance Degradation**: Gradual performance decline requiring investigation
- **Capacity Thresholds**: Resource utilization approaching limits
- **Network Issues**: Intermittent connectivity problems
- **Quality Reduction**: Stream quality below acceptable thresholds

#### Information Alerts
- **Configuration Changes**: System configuration modifications
- **Planned Maintenance**: Scheduled maintenance windows and updates
- **Performance Milestones**: Achievement of performance targets
- **Usage Statistics**: Periodic usage summaries and reports

### Automated Response

#### Self-healing Mechanisms
```python
# Automated response script example
def handle_high_cpu_alert(alert):
    if alert.severity == "critical":
        # Scale up infrastructure
        kubernetes_client.scale_deployment("iris-server", replicas=5)
        
        # Reduce quality to preserve performance
        iris_api.adjust_quality_globally(reduction_factor=0.8)
        
        # Notify operations team
        send_notification("High CPU usage - auto-scaling initiated")
```

#### Recovery Automation
- **Service Restart**: Automatic restart of failed services
- **Traffic Rerouting**: Dynamic traffic distribution to healthy instances
- **Quality Adaptation**: Automatic quality reduction under stress
- **Resource Allocation**: Dynamic resource scaling based on demand

## Performance Analysis

### Query Performance Optimization

#### Efficient Metric Queries
```promql
# Optimized queries for dashboard performance

# Average response time over 5 minutes
avg_over_time(iris_response_time_seconds[5m])

# 95th percentile latency
histogram_quantile(0.95, rate(iris_request_duration_seconds_bucket[5m]))

# Error rate calculation
rate(iris_requests_failed_total[5m]) / rate(iris_requests_total[5m])
```

#### Dashboard Performance
- **Query Caching**: Intelligent caching of expensive queries
- **Data Aggregation**: Pre-computed aggregations for common metrics
- **Progressive Loading**: Incremental dashboard loading for better UX
- **Mobile Optimization**: Responsive dashboards for mobile access

### Capacity Planning

#### Resource Forecasting
```python
# Capacity planning analysis
def predict_resource_needs(historical_data, growth_rate):
    current_usage = historical_data.latest()
    projected_usage = current_usage * (1 + growth_rate) ** months_ahead
    
    return {
        'cpu_cores_needed': math.ceil(projected_usage.cpu * 1.2),
        'memory_gb_needed': math.ceil(projected_usage.memory * 1.2),
        'storage_tb_needed': math.ceil(projected_usage.storage * 1.2)
    }
```

#### Growth Pattern Analysis
- **Usage Trends**: Long-term usage pattern identification
- **Seasonal Variations**: Periodic usage fluctuation analysis
- **Performance Correlation**: Resource usage vs. performance correlation
- **Cost Optimization**: Resource efficiency and cost analysis

## Integration and Extensibility

### API Integration

#### Prometheus API
```python
# Custom metrics collection
from prometheus_client import CollectorRegistry, Gauge, push_to_gateway

registry = CollectorRegistry()
custom_metric = Gauge('iris_custom_quality_score', 
                     'Custom quality score', registry=registry)

custom_metric.set(calculate_quality_score())
push_to_gateway('172.22.0.104:9091', job='custom_metrics', registry=registry)
```

#### Grafana API
```javascript
// Programmatic dashboard creation
const dashboard = {
  dashboard: {
    title: "Dynamic Performance Dashboard",
    panels: generatePanels(streamIds),
    time: { from: "now-1h", to: "now" }
  }
};

await grafanaAPI.createDashboard(dashboard);
```

### Third-party Integrations

#### External Monitoring Systems
- **Datadog**: Enterprise monitoring platform integration
- **New Relic**: Application performance monitoring
- **Elastic Stack**: Log aggregation and analysis
- **Splunk**: Enterprise data platform integration

#### CI/CD Integration
```yaml
# Jenkins pipeline integration
pipeline {
  stages {
    stage('Deploy') {
      steps {
        sh 'kubectl apply -f monitoring-stack.yaml'
        
        // Wait for services to be ready
        sh 'kubectl wait --for=condition=ready pod -l app=prometheus'
        sh 'kubectl wait --for=condition=ready pod -l app=grafana'
        
        // Import dashboards
        sh 'curl -X POST grafana-api/dashboards/import -d @dashboards.json'
      }
    }
  }
}
```

## Security and Access Control

### Authentication and Authorization

#### Grafana Security
```ini
# grafana.ini
[auth]
disable_login_form = false
disable_signout_menu = false

[auth.ldap]
enabled = true
config_file = /etc/grafana/ldap.toml

[security]
admin_user = admin
admin_password = $__env{GF_SECURITY_ADMIN_PASSWORD}
secret_key = $__env{GF_SECURITY_SECRET_KEY}
```

#### Role-based Access
- **Administrator**: Full system access and configuration
- **Operator**: Dashboard access and basic alert management
- **Viewer**: Read-only access to dashboards and metrics
- **Guest**: Limited access to public dashboards

#### Data Privacy
- **Metric Anonymization**: Removal of personally identifiable information
- **Access Logging**: Comprehensive audit trail of data access
- **Data Retention**: Configurable data retention policies
- **Encryption**: Transport and storage encryption for sensitive metrics

### Network Security

#### Service Communication
```yaml
# TLS configuration for Prometheus
global:
  external_labels:
    monitor: 'iris-monitor'
    
tls_config:
  cert_file: /etc/prometheus/prometheus.crt
  key_file: /etc/prometheus/prometheus.key
  ca_file: /etc/prometheus/ca.crt
```

#### Firewall Configuration
- **Port Restrictions**: Minimal port exposure for security
- **Network Segmentation**: Isolated monitoring network
- **VPN Access**: Secure remote access to monitoring interfaces
- **Certificate Management**: Automated certificate renewal and management

## Deployment and Operations

### Container Orchestration

#### Docker Compose Deployment
```yaml
version: '3.8'
services:
  grafana:
    image: grafana/grafana:latest
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${ADMIN_PASSWORD}
    volumes:
      - grafana-storage:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning
    networks:
      iris-network:
        ipv4_address: 172.22.0.103
        
  prometheus:
    image: prom/prometheus:latest
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-storage:/prometheus
    networks:
      iris-network:
        ipv4_address: 172.22.0.104

volumes:
  grafana-storage:
  prometheus-storage:
```

#### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: prometheus
spec:
  replicas: 1
  selector:
    matchLabels:
      app: prometheus
  template:
    spec:
      containers:
      - name: prometheus
        image: prom/prometheus:latest
        ports:
        - containerPort: 9090
        volumeMounts:
        - name: config
          mountPath: /etc/prometheus
        - name: storage
          mountPath: /prometheus
      volumes:
      - name: config
        configMap:
          name: prometheus-config
      - name: storage
        persistentVolumeClaim:
          claimName: prometheus-storage
```

### Backup and Recovery

#### Data Backup Strategy
```bash
#!/bin/bash
# Prometheus backup script
BACKUP_DIR="/backup/prometheus/$(date +%Y%m%d)"
PROMETHEUS_DATA="/prometheus"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Stop Prometheus for consistent backup
docker stop prometheus

# Create backup archive
tar -czf "$BACKUP_DIR/prometheus-data.tar.gz" -C "$PROMETHEUS_DATA" .

# Restart Prometheus
docker start prometheus

# Cleanup old backups (keep 30 days)
find /backup/prometheus -type d -mtime +30 -exec rm -rf {} \;
```

#### Disaster Recovery
- **Automated Backups**: Regular automated backup of time series data
- **Configuration Backup**: Version control for all configuration files
- **Recovery Testing**: Regular disaster recovery testing procedures
- **Multi-site Replication**: Geographic redundancy for critical metrics

## Troubleshooting and Maintenance

### Common Issues and Solutions

#### High Memory Usage
```bash
# Memory optimization for Prometheus
prometheus \
  --config.file=/etc/prometheus/prometheus.yml \
  --storage.tsdb.retention.time=15d \
  --storage.tsdb.retention.size=10GB \
  --storage.tsdb.wal-compression
```

#### Query Performance Issues
```promql
# Optimize expensive queries
# Instead of: sum(rate(iris_packets_total[5m])) by (instance)
# Use: sum(iris:packet_rate_5m) by (instance)
```

#### Dashboard Loading Issues
- **Query Optimization**: Simplify complex queries
- **Time Range Limitation**: Restrict default time ranges
- **Panel Optimization**: Reduce number of concurrent queries
- **Caching**: Implement query result caching

### Maintenance Procedures

#### Regular Maintenance Tasks
- **Index Cleanup**: Regular cleanup of time series indexes
- **Configuration Validation**: Automated configuration testing
- **Performance Tuning**: Regular performance optimization
- **Security Updates**: Automated security patch management

#### Capacity Management
```python
# Automated capacity monitoring
def check_storage_capacity():
    usage = get_disk_usage('/prometheus')
    if usage.percent > 80:
        # Trigger cleanup or expansion
        cleanup_old_data(days=7)
        send_alert("Storage capacity warning", usage.percent)
```

## Future Enhancements

### Advanced Analytics

#### Machine Learning Integration
- **Anomaly Detection**: ML-based anomaly detection for metrics
- **Predictive Analytics**: Forecasting of system behavior
- **Root Cause Analysis**: Automated problem identification
- **Performance Optimization**: AI-driven performance tuning

#### Real-time Processing
- **Stream Processing**: Real-time metric stream processing
- **Complex Event Processing**: Pattern detection in metric streams
- **Dynamic Alerting**: Context-aware alert generation
- **Automated Response**: Intelligent automated problem resolution

### Visualization Enhancements

#### Advanced Dashboards
- **3D Visualization**: Three-dimensional network topology visualization
- **Interactive Analytics**: Dynamic drilling and filtering capabilities
- **Mobile Applications**: Native mobile monitoring applications
- **AR/VR Interfaces**: Immersive monitoring experiences

#### Custom Plugins
- **Network Topology**: Custom network visualization plugins
- **Video Quality**: Specialized video quality assessment panels
- **Device Mapping**: Geographic device distribution visualization
- **Performance Correlation**: Cross-metric correlation analysis tools
