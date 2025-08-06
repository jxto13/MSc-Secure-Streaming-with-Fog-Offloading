# Utilities and Development Tools

## Overview

The utilities directory contains essential development tools, testing scripts, and supporting infrastructure for the IRIS streaming system. These tools facilitate development, testing, performance analysis, and system debugging across the entire platform.

## Directory Structure

```
utils/
├── benchmark.html           # Performance benchmarking web interface
├── clients_random_loss_settings.json  # Network simulation configuration
├── clients_random_loss.sh   # Network loss simulation script
├── protodot                 # Protocol buffer visualization tool
├── testfile.bin            # Binary test data for validation
├── veth_namespace.sh       # Network namespace setup script
└── tcgui/                  # Traffic control GUI application
    ├── src/
    ├── package.json
    └── README.md
```

## Performance Benchmarking

### Benchmark Web Interface

The `benchmark.html` file provides a comprehensive web-based interface for performance testing and analysis:

```html
<!DOCTYPE html>
<html>
<head>
    <title>IRIS Performance Benchmark</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
</head>
<body>
    <div id="throughput-chart"></div>
    <div id="latency-chart"></div>
    <div id="packet-loss-chart"></div>
    
    <script>
        // Real-time performance visualization
        function updateCharts() {
            fetch('/api/metrics')
                .then(response => response.json())
                .then(data => {
                    updateThroughputChart(data.throughput);
                    updateLatencyChart(data.latency);
                    updatePacketLossChart(data.packet_loss);
                });
        }
        
        setInterval(updateCharts, 1000);
    </script>
</body>
</html>
```

#### Features
- **Real-time Metrics**: Live performance data visualization
- **Interactive Charts**: Plotly.js-based interactive charts
- **Export Capabilities**: Data export for offline analysis
- **Comparison Tools**: Side-by-side performance comparisons

#### Metrics Tracked
- **Throughput**: Bits per second across all transmission modes
- **Latency**: End-to-end packet delivery times
- **Packet Loss**: Loss rates and recovery statistics
- **Quality Metrics**: Video quality assessment scores

### Benchmark Usage

```bash
# Start benchmark server
python3 -m http.server 8000 --directory utils/

# Open web interface
firefox http://localhost:8000/benchmark.html

# Run automated benchmark suite
./run_benchmarks.sh --duration 300 --clients 10
```

## Network Simulation Tools

### Random Loss Simulation

The network loss simulation tools enable controlled testing of error correction and recovery mechanisms:

#### Configuration File (`clients_random_loss_settings.json`)

```json
{
  "simulation_settings": {
    "duration_seconds": 300,
    "loss_patterns": [
      {
        "name": "light_loss",
        "packet_loss_rate": 0.01,
        "burst_loss_probability": 0.05,
        "burst_length_mean": 3
      },
      {
        "name": "moderate_loss",
        "packet_loss_rate": 0.05,
        "burst_loss_probability": 0.10,
        "burst_length_mean": 5
      },
      {
        "name": "heavy_loss",
        "packet_loss_rate": 0.15,
        "burst_loss_probability": 0.20,
        "burst_length_mean": 10
      }
    ],
    "network_conditions": [
      {
        "name": "wifi_good",
        "bandwidth_mbps": 54,
        "latency_ms": 5,
        "jitter_ms": 2
      },
      {
        "name": "wifi_congested",
        "bandwidth_mbps": 20,
        "latency_ms": 15,
        "jitter_ms": 8
      },
      {
        "name": "cellular_4g",
        "bandwidth_mbps": 10,
        "latency_ms": 50,
        "jitter_ms": 20
      }
    ]
  }
}
```

#### Loss Simulation Script (`clients_random_loss.sh`)

```bash
#!/bin/bash

# IRIS Network Loss Simulation Script
# Simulates various network conditions for testing streaming resilience

set -e

CONFIG_FILE="clients_random_loss_settings.json"
LOG_DIR="./simulation_logs"
RESULTS_DIR="./simulation_results"

# Load configuration
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Configuration file not found: $CONFIG_FILE"
    exit 1
fi

# Parse JSON configuration
DURATION=$(jq -r '.simulation_settings.duration_seconds' "$CONFIG_FILE")
LOSS_PATTERNS=($(jq -r '.simulation_settings.loss_patterns[].name' "$CONFIG_FILE"))
NETWORK_CONDITIONS=($(jq -r '.simulation_settings.network_conditions[].name' "$CONFIG_FILE"))

echo "Starting IRIS network simulation"
echo "Duration: ${DURATION} seconds"
echo "Loss patterns: ${LOSS_PATTERNS[@]}"
echo "Network conditions: ${NETWORK_CONDITIONS[@]}"

# Create output directories
mkdir -p "$LOG_DIR" "$RESULTS_DIR"

# Run simulation for each combination
for loss_pattern in "${LOSS_PATTERNS[@]}"; do
    for network_condition in "${NETWORK_CONDITIONS[@]}"; do
        echo "Running simulation: $loss_pattern + $network_condition"
        
        # Extract parameters
        LOSS_RATE=$(jq -r ".simulation_settings.loss_patterns[] | select(.name == \"$loss_pattern\") | .packet_loss_rate" "$CONFIG_FILE")
        BANDWIDTH=$(jq -r ".simulation_settings.network_conditions[] | select(.name == \"$network_condition\") | .bandwidth_mbps" "$CONFIG_FILE")
        LATENCY=$(jq -r ".simulation_settings.network_conditions[] | select(.name == \"$network_condition\") | .latency_ms" "$CONFIG_FILE")
        
        # Setup network namespace
        ./veth_namespace.sh create "test_${loss_pattern}_${network_condition}"
        
        # Apply network conditions
        tc qdisc add dev veth1 root handle 1: netem \
            delay "${LATENCY}ms" \
            loss "${LOSS_RATE}%" \
            rate "${BANDWIDTH}mbit"
        
        # Start clients and collect metrics
        LOG_FILE="$LOG_DIR/${loss_pattern}_${network_condition}.log"
        RESULT_FILE="$RESULTS_DIR/${loss_pattern}_${network_condition}.json"
        
        # Run test clients
        timeout $DURATION ./run_test_clients.sh \
            --namespace "test_${loss_pattern}_${network_condition}" \
            --log-file "$LOG_FILE" \
            --result-file "$RESULT_FILE"
        
        # Cleanup
        ./veth_namespace.sh destroy "test_${loss_pattern}_${network_condition}"
        
        echo "Completed: $loss_pattern + $network_condition"
    done
done

echo "Simulation complete. Results in $RESULTS_DIR"
```

#### Key Features
- **Configurable Loss Patterns**: Various packet loss scenarios
- **Network Condition Simulation**: Different bandwidth and latency conditions
- **Automated Execution**: Scripted test execution and data collection
- **Result Aggregation**: Structured output for analysis

### Network Namespace Management

The `veth_namespace.sh` script provides network isolation for testing:

```bash
#!/bin/bash

# Network namespace management for IRIS testing
# Creates isolated network environments for controlled testing

NAMESPACE_PREFIX="iris_test"

create_namespace() {
    local name="$1"
    local namespace="${NAMESPACE_PREFIX}_${name}"
    
    echo "Creating network namespace: $namespace"
    
    # Create namespace
    ip netns add "$namespace"
    
    # Create veth pair
    ip link add "veth_${name}_0" type veth peer name "veth_${name}_1"
    
    # Move one end to namespace
    ip link set "veth_${name}_1" netns "$namespace"
    
    # Configure interfaces
    ip addr add "10.0.${name//[^0-9]/}.1/24" dev "veth_${name}_0"
    ip link set "veth_${name}_0" up
    
    # Configure namespace interface
    ip netns exec "$namespace" ip addr add "10.0.${name//[^0-9]/}.2/24" dev "veth_${name}_1"
    ip netns exec "$namespace" ip link set "veth_${name}_1" up
    ip netns exec "$namespace" ip link set lo up
    
    echo "Namespace $namespace created successfully"
}

destroy_namespace() {
    local name="$1"
    local namespace="${NAMESPACE_PREFIX}_${name}"
    
    echo "Destroying network namespace: $namespace"
    
    # Remove veth pair
    ip link delete "veth_${name}_0" 2>/dev/null || true
    
    # Remove namespace
    ip netns delete "$namespace" 2>/dev/null || true
    
    echo "Namespace $namespace destroyed"
}

list_namespaces() {
    echo "Active IRIS test namespaces:"
    ip netns list | grep "^${NAMESPACE_PREFIX}_" || echo "No namespaces found"
}

case "$1" in
    create)
        create_namespace "$2"
        ;;
    destroy)
        destroy_namespace "$2"
        ;;
    list)
        list_namespaces
        ;;
    *)
        echo "Usage: $0 {create|destroy|list} [namespace_name]"
        exit 1
        ;;
esac
```

## Protocol Visualization

### Protocol Buffer Visualization Tool

The `protodot` tool generates visual representations of Protocol Buffer definitions:

```bash
#!/usr/bin/env python3

"""
Protocol Buffer visualization tool for IRIS
Generates graphical representations of .proto files
"""

import sys
import os
import subprocess
import argparse
from pathlib import Path

def generate_dot_file(proto_file, output_file):
    """Generate DOT file from Protocol Buffer definition"""
    
    # Parse proto file and extract message definitions
    messages = []
    services = []
    
    with open(proto_file, 'r') as f:
        content = f.read()
        
        # Extract messages
        import re
        message_pattern = r'message\s+(\w+)\s*\{([^}]+)\}'
        for match in re.finditer(message_pattern, content, re.MULTILINE | re.DOTALL):
            message_name = match.group(1)
            message_body = match.group(2)
            
            fields = []
            field_pattern = r'(\w+)\s+(\w+)\s*=\s*(\d+);'
            for field_match in re.finditer(field_pattern, message_body):
                field_type = field_match.group(1)
                field_name = field_match.group(2)
                field_number = field_match.group(3)
                fields.append((field_type, field_name, field_number))
            
            messages.append((message_name, fields))
        
        # Extract services
        service_pattern = r'service\s+(\w+)\s*\{([^}]+)\}'
        for match in re.finditer(service_pattern, content, re.MULTILINE | re.DOTALL):
            service_name = match.group(1)
            service_body = match.group(2)
            
            rpcs = []
            rpc_pattern = r'rpc\s+(\w+)\s*\((\w+)\)\s*returns\s*\((\w+)\);'
            for rpc_match in re.finditer(rpc_pattern, service_body):
                rpc_name = rpc_match.group(1)
                request_type = rpc_match.group(2)
                response_type = rpc_match.group(3)
                rpcs.append((rpc_name, request_type, response_type))
            
            services.append((service_name, rpcs))
    
    # Generate DOT content
    dot_content = generate_dot_content(messages, services)
    
    with open(output_file, 'w') as f:
        f.write(dot_content)

def generate_dot_content(messages, services):
    """Generate DOT graph content"""
    
    lines = []
    lines.append("digraph iris_protocol {")
    lines.append("    rankdir=LR;")
    lines.append("    node [shape=record];")
    lines.append("")
    
    # Generate message nodes
    for message_name, fields in messages:
        field_lines = []
        for field_type, field_name, field_number in fields:
            field_lines.append(f"{field_name}: {field_type}")
        
        field_str = "|".join(field_lines)
        lines.append(f'    {message_name} [label="{message_name}|{field_str}"];')
    
    lines.append("")
    
    # Generate service nodes
    for service_name, rpcs in services:
        rpc_lines = []
        for rpc_name, request_type, response_type in rpcs:
            rpc_lines.append(f"{rpc_name}({request_type}) -> {response_type}")
        
        rpc_str = "|".join(rpc_lines)
        lines.append(f'    {service_name} [shape=ellipse, label="{service_name}|{rpc_str}"];')
    
    lines.append("")
    
    # Generate relationships
    for service_name, rpcs in services:
        for rpc_name, request_type, response_type in rpcs:
            lines.append(f'    {request_type} -> {service_name} [label="{rpc_name}"];')
            lines.append(f'    {service_name} -> {response_type} [label="{rpc_name}"];')
    
    lines.append("}")
    return "\n".join(lines)

def main():
    parser = argparse.ArgumentParser(description='Generate protocol visualization')
    parser.add_argument('proto_file', help='Input .proto file')
    parser.add_argument('-o', '--output', help='Output file (PNG/SVG)')
    parser.add_argument('-f', '--format', default='png', choices=['png', 'svg', 'dot'])
    
    args = parser.parse_args()
    
    # Generate DOT file
    dot_file = f"{args.proto_file}.dot"
    generate_dot_file(args.proto_file, dot_file)
    
    if args.format == 'dot':
        if args.output:
            os.rename(dot_file, args.output)
        print(f"DOT file generated: {args.output or dot_file}")
        return
    
    # Generate image
    output_file = args.output or f"{args.proto_file}.{args.format}"
    subprocess.run(['dot', f'-T{args.format}', dot_file, '-o', output_file])
    
    # Cleanup
    os.remove(dot_file)
    
    print(f"Protocol visualization generated: {output_file}")

if __name__ == '__main__':
    main()
```

#### Usage Examples

```bash
# Generate PNG visualization
./protodot ../iris/proto/streaming_server.proto -o streaming_protocol.png

# Generate SVG for web embedding
./protodot ../iris/proto/edge_client.proto -f svg -o mesh_protocol.svg

# Generate DOT file for further editing
./protodot ../iris/proto/streaming_server.proto -f dot -o protocol.dot
```

## Traffic Control GUI

### Web-based Traffic Control Interface

The `tcgui` directory contains a web-based interface for network traffic control:

```
tcgui/
├── package.json        # Node.js dependencies
├── src/
│   ├── index.html     # Main interface
│   ├── app.js         # Application logic
│   ├── styles.css     # Interface styling
│   └── api.js         # Backend API integration
└── README.md          # Usage instructions
```

#### Main Interface (`src/index.html`)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>IRIS Traffic Control</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <div class="container">
        <h1>IRIS Network Traffic Control</h1>
        
        <div class="interface-selection">
            <label for="interface">Network Interface:</label>
            <select id="interface">
                <option value="">Select interface...</option>
            </select>
        </div>
        
        <div class="controls">
            <div class="control-group">
                <h3>Bandwidth Control</h3>
                <label for="bandwidth">Bandwidth (Mbps):</label>
                <input type="number" id="bandwidth" min="1" max="1000" value="100">
                <button onclick="setBandwidth()">Apply</button>
            </div>
            
            <div class="control-group">
                <h3>Latency Simulation</h3>
                <label for="latency">Latency (ms):</label>
                <input type="number" id="latency" min="0" max="1000" value="0">
                <label for="jitter">Jitter (ms):</label>
                <input type="number" id="jitter" min="0" max="100" value="0">
                <button onclick="setLatency()">Apply</button>
            </div>
            
            <div class="control-group">
                <h3>Packet Loss</h3>
                <label for="loss-rate">Loss Rate (%):</label>
                <input type="number" id="loss-rate" min="0" max="50" step="0.1" value="0">
                <label for="burst-loss">Burst Loss:</label>
                <input type="checkbox" id="burst-loss">
                <button onclick="setPacketLoss()">Apply</button>
            </div>
        </div>
        
        <div class="presets">
            <h3>Network Condition Presets</h3>
            <button onclick="applyPreset('perfect')">Perfect</button>
            <button onclick="applyPreset('wifi-good')">WiFi Good</button>
            <button onclick="applyPreset('wifi-congested')">WiFi Congested</button>
            <button onclick="applyPreset('cellular-4g')">Cellular 4G</button>
            <button onclick="applyPreset('poor-connection')">Poor Connection</button>
        </div>
        
        <div class="status">
            <h3>Current Configuration</h3>
            <pre id="current-config">No configuration applied</pre>
        </div>
        
        <div class="actions">
            <button onclick="clearAllRules()" class="danger">Clear All Rules</button>
            <button onclick="saveConfiguration()">Save Configuration</button>
            <button onclick="loadConfiguration()">Load Configuration</button>
        </div>
    </div>
    
    <script src="app.js"></script>
</body>
</html>
```

#### Application Logic (`src/app.js`)

```javascript
// Traffic Control GUI Application Logic

class TrafficController {
    constructor() {
        this.currentInterface = null;
        this.loadInterfaces();
    }
    
    async loadInterfaces() {
        try {
            const response = await fetch('/api/interfaces');
            const interfaces = await response.json();
            
            const select = document.getElementById('interface');
            select.innerHTML = '<option value="">Select interface...</option>';
            
            interfaces.forEach(iface => {
                const option = document.createElement('option');
                option.value = iface.name;
                option.textContent = `${iface.name} (${iface.ip})`;
                select.appendChild(option);
            });
            
            select.addEventListener('change', (e) => {
                this.currentInterface = e.target.value;
                this.updateCurrentConfig();
            });
        } catch (error) {
            console.error('Failed to load interfaces:', error);
        }
    }
    
    async setBandwidth() {
        const bandwidth = document.getElementById('bandwidth').value;
        if (!this.currentInterface || !bandwidth) return;
        
        try {
            await fetch('/api/bandwidth', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    interface: this.currentInterface,
                    bandwidth: parseInt(bandwidth)
                })
            });
            
            this.updateCurrentConfig();
            this.showNotification('Bandwidth limit applied');
        } catch (error) {
            this.showError('Failed to set bandwidth: ' + error.message);
        }
    }
    
    async setLatency() {
        const latency = document.getElementById('latency').value;
        const jitter = document.getElementById('jitter').value;
        if (!this.currentInterface) return;
        
        try {
            await fetch('/api/latency', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    interface: this.currentInterface,
                    latency: parseInt(latency),
                    jitter: parseInt(jitter)
                })
            });
            
            this.updateCurrentConfig();
            this.showNotification('Latency simulation applied');
        } catch (error) {
            this.showError('Failed to set latency: ' + error.message);
        }
    }
    
    async setPacketLoss() {
        const lossRate = document.getElementById('loss-rate').value;
        const burstLoss = document.getElementById('burst-loss').checked;
        if (!this.currentInterface) return;
        
        try {
            await fetch('/api/packet-loss', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    interface: this.currentInterface,
                    loss_rate: parseFloat(lossRate),
                    burst_loss: burstLoss
                })
            });
            
            this.updateCurrentConfig();
            this.showNotification('Packet loss simulation applied');
        } catch (error) {
            this.showError('Failed to set packet loss: ' + error.message);
        }
    }
    
    async applyPreset(presetName) {
        const presets = {
            'perfect': {
                bandwidth: 1000,
                latency: 0,
                jitter: 0,
                loss_rate: 0
            },
            'wifi-good': {
                bandwidth: 54,
                latency: 5,
                jitter: 2,
                loss_rate: 0.1
            },
            'wifi-congested': {
                bandwidth: 20,
                latency: 15,
                jitter: 8,
                loss_rate: 2.0
            },
            'cellular-4g': {
                bandwidth: 10,
                latency: 50,
                jitter: 20,
                loss_rate: 1.0
            },
            'poor-connection': {
                bandwidth: 2,
                latency: 200,
                jitter: 50,
                loss_rate: 10.0
            }
        };
        
        const preset = presets[presetName];
        if (!preset || !this.currentInterface) return;
        
        try {
            await fetch('/api/apply-preset', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    interface: this.currentInterface,
                    ...preset
                })
            });
            
            // Update UI to reflect preset values
            document.getElementById('bandwidth').value = preset.bandwidth;
            document.getElementById('latency').value = preset.latency;
            document.getElementById('jitter').value = preset.jitter;
            document.getElementById('loss-rate').value = preset.loss_rate;
            
            this.updateCurrentConfig();
            this.showNotification(`Applied preset: ${presetName}`);
        } catch (error) {
            this.showError('Failed to apply preset: ' + error.message);
        }
    }
    
    async updateCurrentConfig() {
        if (!this.currentInterface) return;
        
        try {
            const response = await fetch(`/api/config/${this.currentInterface}`);
            const config = await response.json();
            
            document.getElementById('current-config').textContent = 
                JSON.stringify(config, null, 2);
        } catch (error) {
            console.error('Failed to fetch current config:', error);
        }
    }
    
    async clearAllRules() {
        if (!this.currentInterface) return;
        
        if (!confirm('Are you sure you want to clear all traffic control rules?')) {
            return;
        }
        
        try {
            await fetch(`/api/clear/${this.currentInterface}`, {
                method: 'DELETE'
            });
            
            this.updateCurrentConfig();
            this.showNotification('All rules cleared');
        } catch (error) {
            this.showError('Failed to clear rules: ' + error.message);
        }
    }
    
    showNotification(message) {
        // Simple notification implementation
        const notification = document.createElement('div');
        notification.className = 'notification success';
        notification.textContent = message;
        document.body.appendChild(notification);
        
        setTimeout(() => {
            notification.remove();
        }, 3000);
    }
    
    showError(message) {
        const notification = document.createElement('div');
        notification.className = 'notification error';
        notification.textContent = message;
        document.body.appendChild(notification);
        
        setTimeout(() => {
            notification.remove();
        }, 5000);
    }
}

// Initialize application
const trafficController = new TrafficController();

// Expose functions to global scope for onclick handlers
window.setBandwidth = () => trafficController.setBandwidth();
window.setLatency = () => trafficController.setLatency();
window.setPacketLoss = () => trafficController.setPacketLoss();
window.applyPreset = (preset) => trafficController.applyPreset(preset);
window.clearAllRules = () => trafficController.clearAllRules();
window.saveConfiguration = () => trafficController.saveConfiguration();
window.loadConfiguration = () => trafficController.loadConfiguration();
```

## Test Data and Validation

### Binary Test File

The `testfile.bin` contains structured binary data for testing various system components:

```bash
# Generate test file with known patterns
dd if=/dev/zero of=testfile.bin bs=1024 count=1024
# Add pattern for packet boundary detection
for i in {0..1023}; do
    printf "%04x" $i | xxd -r -p >> testfile.bin
done
```

#### Test File Structure
- **Header Section**: 64 bytes of metadata
- **Pattern Section**: Repeating 4-byte patterns for validation
- **Random Section**: Pseudo-random data for entropy testing
- **Checksum Section**: SHA-256 hash for integrity verification

### Validation Scripts

```bash
#!/bin/bash
# validate_system.sh - Comprehensive system validation

echo "IRIS System Validation"
echo "====================="

# Test network connectivity
echo "Testing network connectivity..."
if ping -c 3 172.22.0.102 > /dev/null; then
    echo "✓ IRIS server reachable"
else
    echo "✗ IRIS server unreachable"
    exit 1
fi

# Test protocol buffer compilation
echo "Testing protocol compilation..."
if protoc --version > /dev/null; then
    echo "✓ Protocol Buffers available"
else
    echo "✗ Protocol Buffers not found"
    exit 1
fi

# Test certificate generation
echo "Testing certificate generation..."
if openssl version > /dev/null; then
    echo "✓ OpenSSL available"
else
    echo "✗ OpenSSL not found"
    exit 1
fi

# Test streaming functionality
echo "Testing streaming functionality..."
timeout 30 ./test_stream.sh
if [ $? -eq 0 ]; then
    echo "✓ Streaming test passed"
else
    echo "✗ Streaming test failed"
fi

echo "Validation complete"
```

## Development Automation

### Build Scripts

```bash
#!/bin/bash
# build_all.sh - Complete system build script

set -e

echo "Building IRIS System Components"
echo "==============================="

# Build Rust components
echo "Building Rust components..."
cargo build --workspace --release

# Build Android APK
echo "Building Android application..."
cd android
./gradlew assembleRelease
cd ..

# Build Docker images
echo "Building Docker images..."
docker-compose -f iris/docker-compose.yml build
docker-compose -f encoder/docker-compose.yml build
docker-compose -f stats/docker-compose.yml build

# Generate documentation
echo "Generating documentation..."
cargo doc --workspace --no-deps

echo "Build complete!"
```

### Testing Automation

```bash
#!/bin/bash
# run_tests.sh - Automated testing suite

TEST_RESULTS_DIR="./test_results/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$TEST_RESULTS_DIR"

echo "Running IRIS Test Suite"
echo "======================="

# Unit tests
echo "Running unit tests..."
cargo test --workspace --release > "$TEST_RESULTS_DIR/unit_tests.log" 2>&1
if [ $? -eq 0 ]; then
    echo "✓ Unit tests passed"
else
    echo "✗ Unit tests failed (see $TEST_RESULTS_DIR/unit_tests.log)"
fi

# Integration tests
echo "Running integration tests..."
./utils/run_integration_tests.sh > "$TEST_RESULTS_DIR/integration_tests.log" 2>&1
if [ $? -eq 0 ]; then
    echo "✓ Integration tests passed"
else
    echo "✗ Integration tests failed (see $TEST_RESULTS_DIR/integration_tests.log)"
fi

# Performance tests
echo "Running performance tests..."
./utils/run_performance_tests.sh > "$TEST_RESULTS_DIR/performance_tests.log" 2>&1

# Network simulation tests
echo "Running network simulation tests..."
./utils/clients_random_loss.sh > "$TEST_RESULTS_DIR/network_simulation.log" 2>&1

echo "Test suite complete. Results in $TEST_RESULTS_DIR"
```

## Monitoring Integration

### Metrics Collection Scripts

```python
#!/usr/bin/env python3
# collect_metrics.py - Automated metrics collection

import json
import time
import requests
import argparse
from datetime import datetime

def collect_prometheus_metrics(prometheus_url):
    """Collect metrics from Prometheus"""
    
    queries = [
        'iris_packets_sent_total',
        'iris_packets_received_total',
        'iris_packet_loss_rate',
        'iris_bandwidth_utilization',
        'iris_fec_recovery_rate'
    ]
    
    metrics = {}
    for query in queries:
        try:
            response = requests.get(f"{prometheus_url}/api/v1/query", 
                                  params={'query': query})
            data = response.json()
            metrics[query] = data['data']['result']
        except Exception as e:
            print(f"Failed to collect {query}: {e}")
            metrics[query] = None
    
    return metrics

def generate_report(metrics, output_file):
    """Generate performance report"""
    
    report = {
        'timestamp': datetime.now().isoformat(),
        'metrics': metrics,
        'summary': {
            'total_packets': sum_metric(metrics.get('iris_packets_sent_total', [])),
            'loss_rate': avg_metric(metrics.get('iris_packet_loss_rate', [])),
            'bandwidth_utilization': avg_metric(metrics.get('iris_bandwidth_utilization', []))
        }
    }
    
    with open(output_file, 'w') as f:
        json.dump(report, f, indent=2)
    
    print(f"Report generated: {output_file}")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--prometheus-url', 
                       default='http://172.22.0.104:9090',
                       help='Prometheus server URL')
    parser.add_argument('--output', 
                       default=f'metrics_report_{int(time.time())}.json',
                       help='Output file path')
    
    args = parser.parse_args()
    
    metrics = collect_prometheus_metrics(args.prometheus_url)
    generate_report(metrics, args.output)

if __name__ == '__main__':
    main()
```

## Documentation Generation

### Automated Documentation

```bash
#!/bin/bash
# generate_docs.sh - Comprehensive documentation generation

DOC_DIR="./generated_docs"
mkdir -p "$DOC_DIR"

echo "Generating IRIS Documentation"
echo "============================"

# Generate Rust API documentation
echo "Generating Rust API docs..."
cargo doc --workspace --no-deps --target-dir "$DOC_DIR/rust"

# Generate Protocol Buffer documentation
echo "Generating protocol documentation..."
mkdir -p "$DOC_DIR/protocols"
for proto_file in iris/proto/*.proto; do
    filename=$(basename "$proto_file" .proto)
    ./utils/protodoc "$proto_file" > "$DOC_DIR/protocols/$filename.md"
done

# Generate network diagrams
echo "Generating network diagrams..."
mkdir -p "$DOC_DIR/diagrams"
./utils/protodot iris/proto/streaming_server.proto -o "$DOC_DIR/diagrams/streaming_protocol.png"
./utils/protodot iris/proto/edge_client.proto -o "$DOC_DIR/diagrams/mesh_protocol.png"

# Generate performance reports
echo "Generating performance documentation..."
python3 utils/generate_performance_docs.py --output "$DOC_DIR/performance.md"

# Create index page
cat > "$DOC_DIR/index.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>IRIS Documentation</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .section { margin: 20px 0; }
        .section h2 { color: #333; border-bottom: 2px solid #007acc; }
    </style>
</head>
<body>
    <h1>IRIS Streaming System Documentation</h1>
    
    <div class="section">
        <h2>API Documentation</h2>
        <ul>
            <li><a href="rust/doc/index.html">Rust API Documentation</a></li>
        </ul>
    </div>
    
    <div class="section">
        <h2>Protocol Documentation</h2>
        <ul>
            <li><a href="protocols/streaming_server.md">Streaming Server Protocol</a></li>
            <li><a href="protocols/edge_client.md">Edge Client Protocol</a></li>
        </ul>
    </div>
    
    <div class="section">
        <h2>Network Diagrams</h2>
        <ul>
            <li><a href="diagrams/streaming_protocol.png">Streaming Protocol Diagram</a></li>
            <li><a href="diagrams/mesh_protocol.png">Mesh Protocol Diagram</a></li>
        </ul>
    </div>
    
    <div class="section">
        <h2>Performance Analysis</h2>
        <ul>
            <li><a href="performance.md">Performance Documentation</a></li>
            <li><a href="../utils/benchmark.html">Interactive Benchmark</a></li>
        </ul>
    </div>
</body>
</html>
EOF

echo "Documentation generated in $DOC_DIR"
echo "Open $DOC_DIR/index.html to view"
```

## Future Development Tools

### Planned Utilities

- **AI-powered Testing**: Machine learning-based test case generation
- **Visual Protocol Designer**: Drag-and-drop protocol definition tool
- **Performance Profiler**: Real-time performance analysis and optimization suggestions
- **Security Scanner**: Automated security vulnerability assessment
- **Deployment Wizard**: Guided deployment configuration tool

### Integration Capabilities

- **CI/CD Integration**: Jenkins, GitHub Actions, GitLab CI integration
- **Monitoring Integration**: Enhanced Prometheus and Grafana integration
- **Cloud Deployment**: AWS, Azure, GCP deployment automation
- **Container Orchestration**: Kubernetes and Docker Swarm support

The utilities in this directory provide a comprehensive toolkit for developing, testing, and maintaining the IRIS streaming system, enabling efficient development workflows and robust system validation.
