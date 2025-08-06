# WiFi Aware Library

## Overview

The WiFi Aware library provides a comprehensive Java wrapper around Android's Neighbor Awareness Networking (NAN) API. This library abstracts the complexity of WiFi Aware protocol implementation while providing robust device discovery, connection management, and data path establishment capabilities for the IRIS streaming system.

## Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ WifiAwareLibrary│    │ Discovery Session│    │  Data Path      │
│                 │    │                 │    │                 │
│ • Session Mgmt  │◄──►│ • Publishing    │◄──►│ • IPv6 Address  │
│ • Callback Mgmt │    │ • Subscribing   │    │ • Network Config│
│ • State Tracking│    │ • Peer Discovery│    │ • Data Transfer │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         v                       v                       v
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Android NAN    │    │  Network Stack  │    │   Application   │
│                 │    │                 │    │                 │
│ • Hardware API  │    │ • IPv6 Setup    │    │ • Stream Client │
│ • Radio Control │    │ • Routing       │    │ • Mesh Network  │
│ • Power Mgmt    │    │ • Interface Mgmt│    │ • Video Playback│
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Core Implementation

### WifiAwareLibrary Class

```java
package pt.iris.network.lib;

import android.content.Context;
import android.net.wifi.aware.*;
import android.net.ConnectivityManager;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.util.Log;

import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;

@SuppressLint("MissingPermission")
public class WifiAwareLibrary {
    private static final String TAG = "WifiAwareLibrary";
    private static final String SERVICE_NAME = "iris_streaming";
    private static final int DISCOVERY_PORT = 9060;
    
    private final Context context;
    private final WifiAwareManager wifiAwareManager;
    private final ConnectivityManager connectivityManager;
    
    // Session management
    private WifiAwareSession currentSession;
    private PublishDiscoverySession publishSession;
    private SubscribeDiscoverySession subscribeSession;
    
    // State tracking
    private final Map<PeerHandle, WifiAwareModels.DataPathInfo> activeConnections;
    private final List<WifiAwareCallback> callbacks;
    private WifiAwareModels.DeviceRole currentRole;
    private boolean isWifiAwareAvailable;
    
    // Configuration
    private PublishConfig publishConfig;
    private SubscribeConfig subscribeConfig;
    
    public WifiAwareLibrary(Context context) {
        this.context = context;
        this.wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);
        this.connectivityManager = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
        
        this.activeConnections = new ConcurrentHashMap<>();
        this.callbacks = new CopyOnWriteArrayList<>();
        this.currentRole = WifiAwareModels.DeviceRole.NONE;
        this.isWifiAwareAvailable = false;
        
        initializeConfigurations();
        checkWifiAwareAvailability();
    }
}
```

### Device Role Management

```java
public enum DeviceRole {
    NONE("Not connected"),
    PUBLISHER("Publishing service"),
    SUBSCRIBER("Subscribing to service"),
    BOTH("Publishing and subscribing");
    
    private final String description;
    
    DeviceRole(String description) {
        this.description = description;
    }
    
    public String getDescription() {
        return description;
    }
}

private void updateRole() {
    WifiAwareModels.DeviceRole newRole;
    
    if (publishSession != null && subscribeSession != null) {
        newRole = WifiAwareModels.DeviceRole.BOTH;
    } else if (publishSession != null) {
        newRole = WifiAwareModels.DeviceRole.PUBLISHER;
    } else if (subscribeSession != null) {
        newRole = WifiAwareModels.DeviceRole.SUBSCRIBER;
    } else {
        newRole = WifiAwareModels.DeviceRole.NONE;
    }
    
    if (newRole != currentRole) {
        currentRole = newRole;
        notifyRoleChanged(newRole);
    }
}
```

## Service Discovery

### Publishing Implementation

```java
public void startPublishing() {
    if (!isWifiAwareAvailable || currentSession == null) {
        Log.w(TAG, "Cannot start publishing: WiFi Aware not available");
        return;
    }
    
    if (publishSession != null) {
        Log.d(TAG, "Already publishing");
        return;
    }
    
    Log.d(TAG, "Starting WiFi Aware publishing");
    currentSession.publish(publishConfig, new DiscoverySessionCallback() {
        @Override
        public void onPublishStarted(PublishDiscoverySession session) {
            Log.d(TAG, "Publishing started successfully");
            publishSession = session;
            updateRole();
            notifyPublishStarted(session);
        }
        
        @Override
        public void onSessionConfigUpdated() {
            Log.d(TAG, "Publish session configuration updated");
        }
        
        @Override
        public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
            Log.d(TAG, "Service discovered from peer: " + peerHandle);
            String info = new String(serviceSpecificInfo, StandardCharsets.UTF_8);
            
            // Notify discovery
            notifyServiceDiscovered(peerHandle, serviceSpecificInfo);
            
            // Automatically initiate data path if configured
            if (shouldAutoConnect(info)) {
                initiateDataPath(peerHandle);
            }
        }
        
        @Override
        public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
            Log.d(TAG, "Message received from " + peerHandle + ": " + new String(message, StandardCharsets.UTF_8));
            notifyMessageReceived(peerHandle, message);
        }
        
        @Override
        public void onSessionTerminated() {
            Log.w(TAG, "Publish session terminated");
            publishSession = null;
            updateRole();
        }
    }, null);
}

private void initializePublishConfig() {
    publishConfig = new PublishConfig.Builder()
        .setServiceName(SERVICE_NAME)
        .setServiceSpecificInfo(createServiceInfo())
        .setRangingEnabled(true)
        .setTerminateNotificationEnabled(true)
        .build();
}

private byte[] createServiceInfo() {
    // Create service-specific information
    String deviceInfo = String.format("{\"device_id\":\"%s\",\"capabilities\":[\"streaming\",\"mesh\"]}", 
                                    getDeviceId());
    return deviceInfo.getBytes(StandardCharsets.UTF_8);
}
```

### Subscribing Implementation

```java
public void startSubscribing() {
    if (!isWifiAwareAvailable || currentSession == null) {
        Log.w(TAG, "Cannot start subscribing: WiFi Aware not available");
        return;
    }
    
    if (subscribeSession != null) {
        Log.d(TAG, "Already subscribing");
        return;
    }
    
    Log.d(TAG, "Starting WiFi Aware subscribing");
    currentSession.subscribe(subscribeConfig, new DiscoverySessionCallback() {
        @Override
        public void onSubscribeStarted(SubscribeDiscoverySession session) {
            Log.d(TAG, "Subscribing started successfully");
            subscribeSession = session;
            updateRole();
            notifySubscribeStarted(session);
        }
        
        @Override
        public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
            Log.d(TAG, "Peer discovered: " + peerHandle);
            String info = new String(serviceSpecificInfo, StandardCharsets.UTF_8);
            
            // Notify discovery
            notifyPeerDiscovered(peerHandle);
            notifyServiceDiscovered(peerHandle, serviceSpecificInfo);
            
            // Send initial message
            sendMessage(peerHandle, "Hello from subscriber");
            
            // Request data path establishment
            if (shouldInitiateConnection(info)) {
                requestDataPath(peerHandle);
            }
        }
        
        @Override
        public void onServiceLost(PeerHandle peerHandle, int reason) {
            Log.d(TAG, "Peer lost: " + peerHandle + ", reason: " + reason);
            notifyPeerLost(peerHandle);
            
            // Cleanup connection if exists
            WifiAwareModels.DataPathInfo dataPath = activeConnections.remove(peerHandle);
            if (dataPath != null) {
                notifyConnectionLost(peerHandle);
            }
        }
        
        @Override
        public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
            Log.d(TAG, "Message received from " + peerHandle + ": " + new String(message, StandardCharsets.UTF_8));
            notifyMessageReceived(peerHandle, message);
        }
        
        @Override
        public void onSessionTerminated() {
            Log.w(TAG, "Subscribe session terminated");
            subscribeSession = null;
            updateRole();
        }
    }, null);
}

private void initializeSubscribeConfig() {
    subscribeConfig = new SubscribeConfig.Builder()
        .setServiceName(SERVICE_NAME)
        .setRangingEnabled(true)
        .setTerminateNotificationEnabled(true)
        .build();
}
```

## Data Path Management

### IPv6 Data Path Establishment

```java
private void initiateDataPath(PeerHandle peerHandle) {
    Log.d(TAG, "Initiating data path with peer: " + peerHandle);
    
    if (publishSession == null) {
        Log.e(TAG, "Cannot initiate data path: no publish session");
        return;
    }
    
    // Create network specifier for data path
    WifiAwareNetworkSpecifier.Builder specifierBuilder = 
        new WifiAwareNetworkSpecifier.Builder(publishSession, peerHandle)
            .setPskPassphrase("iris_secure_passphrase")
            .setPort(DISCOVERY_PORT);
    
    NetworkSpecifier networkSpecifier = specifierBuilder.build();
    
    // Create network request
    NetworkRequest networkRequest = new NetworkRequest.Builder()
        .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
        .setNetworkSpecifier(networkSpecifier)
        .build();
    
    // Request network
    connectivityManager.requestNetwork(networkRequest, new ConnectivityManager.NetworkCallback() {
        @Override
        public void onAvailable(Network network) {
            Log.d(TAG, "Data path network available: " + network);
            handleDataPathAvailable(peerHandle, network);
        }
        
        @Override
        public void onLinkPropertiesChanged(Network network, LinkProperties linkProperties) {
            Log.d(TAG, "Data path link properties changed");
            updateDataPathInfo(peerHandle, network, linkProperties);
        }
        
        @Override
        public void onLost(Network network) {
            Log.d(TAG, "Data path network lost: " + network);
            handleDataPathLost(peerHandle, network);
        }
        
        @Override
        public void onUnavailable() {
            Log.w(TAG, "Data path network unavailable");
            handleDataPathUnavailable(peerHandle);
        }
    });
}

private void handleDataPathAvailable(PeerHandle peerHandle, Network network) {
    try {
        // Get link properties to extract IPv6 address
        LinkProperties linkProperties = connectivityManager.getLinkProperties(network);
        if (linkProperties == null) {
            Log.e(TAG, "No link properties available for data path");
            return;
        }
        
        // Extract IPv6 link-local address
        String ipv6Address = extractIpv6Address(linkProperties);
        if (ipv6Address == null) {
            Log.e(TAG, "No IPv6 address found for data path");
            return;
        }
        
        // Get interface name
        String interfaceName = linkProperties.getInterfaceName();
        
        // Create data path info
        WifiAwareModels.DataPathInfo dataPathInfo = new WifiAwareModels.DataPathInfo(
            ipv6Address,
            interfaceName,
            DISCOVERY_PORT,
            System.currentTimeMillis()
        );
        
        // Store connection
        activeConnections.put(peerHandle, dataPathInfo);
        
        Log.i(TAG, "Data path established: " + peerHandle + " -> " + ipv6Address + "%" + interfaceName);
        
        // Notify connection established
        notifyConnectionEstablished(peerHandle, dataPathInfo);
        
    } catch (Exception e) {
        Log.e(TAG, "Error handling data path available", e);
    }
}

private String extractIpv6Address(LinkProperties linkProperties) {
    for (LinkAddress linkAddress : linkProperties.getLinkAddresses()) {
        InetAddress address = linkAddress.getAddress();
        if (address instanceof Inet6Address) {
            Inet6Address inet6Address = (Inet6Address) address;
            if (inet6Address.isLinkLocalAddress()) {
                return inet6Address.getHostAddress();
            }
        }
    }
    return null;
}
```

### Data Path Information Model

```java
public static class DataPathInfo {
    public final String ipAddress;
    public final String interfaceName;
    public final int port;
    public final long establishedTime;
    public final NetworkCapabilities capabilities;
    
    public DataPathInfo(String ipAddress, String interfaceName, int port, long establishedTime) {
        this.ipAddress = ipAddress;
        this.interfaceName = interfaceName;
        this.port = port;
        this.establishedTime = establishedTime;
        this.capabilities = null;
    }
    
    public String getFullAddress() {
        return ipAddress + "%" + interfaceName;
    }
    
    public long getConnectionDuration() {
        return System.currentTimeMillis() - establishedTime;
    }
    
    @Override
    public String toString() {
        return String.format("DataPath{ip=%s, interface=%s, port=%d, duration=%dms}", 
                           ipAddress, interfaceName, port, getConnectionDuration());
    }
}
```

## Session Management

### WiFi Aware Session Lifecycle

```java
public void initialize() {
    if (!isWifiAwareSupported()) {
        Log.e(TAG, "WiFi Aware not supported on this device");
        return;
    }
    
    if (!isWifiAwareAvailable()) {
        Log.w(TAG, "WiFi Aware not currently available");
        return;
    }
    
    Log.d(TAG, "Initializing WiFi Aware session");
    
    WifiAwareSession.AttachCallback attachCallback = new WifiAwareSession.AttachCallback() {
        @Override
        public void onAttached(WifiAwareSession session) {
            Log.d(TAG, "WiFi Aware session attached");
            currentSession = session;
            isWifiAwareAvailable = true;
            notifySessionAttached(session);
        }
        
        @Override
        public void onAttachFailed() {
            Log.e(TAG, "WiFi Aware session attach failed");
            currentSession = null;
            isWifiAwareAvailable = false;
            notifyError("Failed to attach WiFi Aware session");
        }
    };
    
    // Attach to WiFi Aware
    wifiAwareManager.attach(attachCallback, null);
}

public void cleanup() {
    Log.d(TAG, "Cleaning up WiFi Aware library");
    
    // Stop publishing
    if (publishSession != null) {
        publishSession.close();
        publishSession = null;
    }
    
    // Stop subscribing
    if (subscribeSession != null) {
        subscribeSession.close();
        subscribeSession = null;
    }
    
    // Clear connections
    for (Map.Entry<PeerHandle, WifiAwareModels.DataPathInfo> entry : activeConnections.entrySet()) {
        notifyConnectionLost(entry.getKey());
    }
    activeConnections.clear();
    
    // Close session
    if (currentSession != null) {
        currentSession.close();
        currentSession = null;
    }
    
    isWifiAwareAvailable = false;
    updateRole();
}
```

### Availability Monitoring

```java
public boolean isWifiAwareSupported() {
    return context.getPackageManager().hasSystemFeature(PackageManager.FEATURE_WIFI_AWARE);
}

public boolean isWifiAwareAvailable() {
    if (!isWifiAwareSupported()) {
        return false;
    }
    
    try {
        return wifiAwareManager.isAvailable();
    } catch (Exception e) {
        Log.e(TAG, "Error checking WiFi Aware availability", e);
        return false;
    }
}

private void checkWifiAwareAvailability() {
    if (!isWifiAwareSupported()) {
        Log.w(TAG, "WiFi Aware not supported on this device");
        notifyError("WiFi Aware not supported");
        return;
    }
    
    boolean available = isWifiAwareAvailable();
    if (isWifiAwareAvailable != available) {
        isWifiAwareAvailable = available;
        notifyWifiAwareAvailable(available);
    }
}
```

## Message Communication

### Peer-to-Peer Messaging

```java
public void sendMessage(PeerHandle peerHandle, String message) {
    if (publishSession != null) {
        sendMessageViaSession(publishSession, peerHandle, message);
    } else if (subscribeSession != null) {
        sendMessageViaSession(subscribeSession, peerHandle, message);
    } else {
        Log.w(TAG, "No active session for sending message");
    }
}

private void sendMessageViaSession(DiscoverySession session, PeerHandle peerHandle, String message) {
    try {
        byte[] messageBytes = message.getBytes(StandardCharsets.UTF_8);
        session.sendMessage(peerHandle, generateMessageId(), messageBytes);
        Log.d(TAG, "Message sent to " + peerHandle + ": " + message);
    } catch (Exception e) {
        Log.e(TAG, "Failed to send message to " + peerHandle, e);
    }
}

private int generateMessageId() {
    return (int) (System.currentTimeMillis() & 0xFFFF);
}

public void broadcastMessage(String message) {
    byte[] messageBytes = message.getBytes(StandardCharsets.UTF_8);
    
    // Send via publish session to all discovered subscribers
    if (publishSession != null) {
        // Note: Direct broadcast not available, need to track peers individually
        Log.d(TAG, "Broadcasting message via publish session: " + message);
    }
    
    // Send via subscribe session to discovered publishers
    if (subscribeSession != null) {
        Log.d(TAG, "Broadcasting message via subscribe session: " + message);
    }
}
```

### Service-Specific Information

```java
private byte[] createServiceSpecificInfo() {
    try {
        JSONObject serviceInfo = new JSONObject();
        serviceInfo.put("device_id", getDeviceId());
        serviceInfo.put("service_version", "1.0");
        serviceInfo.put("capabilities", new JSONArray()
            .put("streaming")
            .put("mesh_networking")
            .put("retransmission"));
        serviceInfo.put("timestamp", System.currentTimeMillis());
        
        return serviceInfo.toString().getBytes(StandardCharsets.UTF_8);
    } catch (JSONException e) {
        Log.e(TAG, "Error creating service info", e);
        return "{}".getBytes(StandardCharsets.UTF_8);
    }
}

private boolean shouldAutoConnect(String serviceInfo) {
    try {
        JSONObject info = new JSONObject(serviceInfo);
        JSONArray capabilities = info.optJSONArray("capabilities");
        
        if (capabilities != null) {
            for (int i = 0; i < capabilities.length(); i++) {
                if ("streaming".equals(capabilities.optString(i))) {
                    return true;
                }
            }
        }
    } catch (JSONException e) {
        Log.w(TAG, "Invalid service info JSON", e);
    }
    
    return false;
}
```

## Callback System

### Callback Interface

```java
public interface WifiAwareCallback {
    void onWifiAwareAvailable(boolean available);
    void onSessionAttached(WifiAwareSession session);
    void onPublishStarted(PublishDiscoverySession session);
    void onSubscribeStarted(SubscribeDiscoverySession session);
    void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo);
    void onMessageReceived(PeerHandle peerHandle, byte[] message);
    void onConnectionEstablished(PeerHandle peerHandle, WifiAwareModels.DataPathInfo dataPath);
    void onConnectionLost(PeerHandle peerHandle);
    void onPeerDiscovered(PeerHandle peerHandle);
    void onPeerLost(PeerHandle peerHandle);
    void onRoleChanged(WifiAwareModels.DeviceRole newRole);
    void onError(String error);
}

public void addCallback(WifiAwareCallback callback) {
    callbacks.add(callback);
}

public void removeCallback(WifiAwareCallback callback) {
    callbacks.remove(callback);
}
```

### Callback Notification Methods

```java
private void notifyWifiAwareAvailable(boolean available) {
    for (WifiAwareCallback callback : callbacks) {
        try {
            callback.onWifiAwareAvailable(available);
        } catch (Exception e) {
            Log.e(TAG, "Error in callback", e);
        }
    }
}

private void notifySessionAttached(WifiAwareSession session) {
    for (WifiAwareCallback callback : callbacks) {
        try {
            callback.onSessionAttached(session);
        } catch (Exception e) {
            Log.e(TAG, "Error in callback", e);
        }
    }
}

private void notifyConnectionEstablished(PeerHandle peerHandle, WifiAwareModels.DataPathInfo dataPath) {
    for (WifiAwareCallback callback : callbacks) {
        try {
            callback.onConnectionEstablished(peerHandle, dataPath);
        } catch (Exception e) {
            Log.e(TAG, "Error in callback", e);
        }
    }
}

private void notifyError(String error) {
    for (WifiAwareCallback callback : callbacks) {
        try {
            callback.onError(error);
        } catch (Exception e) {
            Log.e(TAG, "Error in error callback", e);
        }
    }
}
```

## Statistics and Monitoring

### Connection Statistics

```java
public String getWifiAwareStats() {
    StringBuilder stats = new StringBuilder();
    stats.append("WiFi Aware Statistics\n");
    stats.append("====================\n");
    stats.append("Available: ").append(isWifiAwareAvailable).append("\n");
    stats.append("Session: ").append(currentSession != null ? "Active" : "Inactive").append("\n");
    stats.append("Role: ").append(currentRole.getDescription()).append("\n");
    stats.append("Active Connections: ").append(activeConnections.size()).append("\n");
    
    if (!activeConnections.isEmpty()) {
        stats.append("\nConnections:\n");
        for (Map.Entry<PeerHandle, WifiAwareModels.DataPathInfo> entry : activeConnections.entrySet()) {
            WifiAwareModels.DataPathInfo info = entry.getValue();
            stats.append("  - ").append(entry.getKey())
                 .append(" -> ").append(info.getFullAddress())
                 .append(" (").append(info.getConnectionDuration()).append("ms)\n");
        }
    }
    
    return stats.toString();
}

public WifiAwareModels.SessionInfo getSessionInfo() {
    return new WifiAwareModels.SessionInfo(
        isWifiAwareAvailable,
        currentSession != null,
        currentRole,
        activeConnections.size(),
        publishSession != null,
        subscribeSession != null
    );
}
```

### Performance Metrics

```java
public static class SessionInfo {
    public final boolean wifiAwareAvailable;
    public final boolean sessionActive;
    public final DeviceRole currentRole;
    public final int activeConnections;
    public final boolean publishing;
    public final boolean subscribing;
    public final long sessionUptime;
    
    public SessionInfo(boolean wifiAwareAvailable, boolean sessionActive, 
                      DeviceRole currentRole, int activeConnections,
                      boolean publishing, boolean subscribing) {
        this.wifiAwareAvailable = wifiAwareAvailable;
        this.sessionActive = sessionActive;
        this.currentRole = currentRole;
        this.activeConnections = activeConnections;
        this.publishing = publishing;
        this.subscribing = subscribing;
        this.sessionUptime = System.currentTimeMillis();
    }
    
    @Override
    public String toString() {
        return String.format("SessionInfo{available=%s, active=%s, role=%s, connections=%d}", 
                           wifiAwareAvailable, sessionActive, currentRole, activeConnections);
    }
}
```

## Error Handling and Recovery

### Robust Error Handling

```java
private void handleSessionError(String operation, Exception e) {
    Log.e(TAG, "Error in " + operation, e);
    
    // Determine error type and recovery strategy
    if (e instanceof SecurityException) {
        notifyError("Permission denied for " + operation);
    } else if (e instanceof IllegalStateException) {
        Log.w(TAG, "Invalid state for " + operation + ", attempting recovery");
        attemptRecovery();
    } else {
        notifyError("Unexpected error in " + operation + ": " + e.getMessage());
    }
}

private void attemptRecovery() {
    Log.d(TAG, "Attempting WiFi Aware session recovery");
    
    // Close existing sessions
    cleanup();
    
    // Wait briefly before reinitializing
    new Handler(Looper.getMainLooper()).postDelayed(() -> {
        if (isWifiAwareAvailable()) {
            Log.d(TAG, "Reinitializing WiFi Aware session");
            initialize();
        }
    }, 2000);
}
```

### Network State Monitoring

```java
private void monitorNetworkState() {
    // Monitor WiFi state changes that might affect WiFi Aware
    IntentFilter filter = new IntentFilter();
    filter.addAction(WifiManager.WIFI_STATE_CHANGED_ACTION);
    filter.addAction(WifiManager.NETWORK_STATE_CHANGED_ACTION);
    
    context.registerReceiver(new BroadcastReceiver() {
        @Override
        public void onReceive(Context context, Intent intent) {
            String action = intent.getAction();
            
            if (WifiManager.WIFI_STATE_CHANGED_ACTION.equals(action)) {
                int wifiState = intent.getIntExtra(WifiManager.EXTRA_WIFI_STATE, 
                                                 WifiManager.WIFI_STATE_UNKNOWN);
                handleWifiStateChange(wifiState);
            } else if (WifiManager.NETWORK_STATE_CHANGED_ACTION.equals(action)) {
                NetworkInfo networkInfo = intent.getParcelableExtra(WifiManager.EXTRA_NETWORK_INFO);
                handleNetworkStateChange(networkInfo);
            }
        }
    }, filter);
}

private void handleWifiStateChange(int wifiState) {
    switch (wifiState) {
        case WifiManager.WIFI_STATE_ENABLED:
            Log.d(TAG, "WiFi enabled, checking WiFi Aware availability");
            checkWifiAwareAvailability();
            break;
        case WifiManager.WIFI_STATE_DISABLED:
            Log.d(TAG, "WiFi disabled, cleaning up WiFi Aware");
            cleanup();
            isWifiAwareAvailable = false;
            notifyWifiAwareAvailable(false);
            break;
    }
}
```

## Power Management

### Battery Optimization

```java
public void enablePowerSaveMode(boolean enabled) {
    if (enabled) {
        Log.d(TAG, "Enabling power save mode");
        
        // Reduce discovery frequency
        publishConfig = new PublishConfig.Builder(publishConfig)
            .setTerminateNotificationEnabled(true)
            .build();
            
        subscribeConfig = new SubscribeConfig.Builder(subscribeConfig)
            .setTerminateNotificationEnabled(true)
            .build();
    } else {
        Log.d(TAG, "Disabling power save mode");
        initializeConfigurations();
    }
}

public void optimizeForBatteryLife() {
    // Implement battery-aware operation
    PowerManager powerManager = (PowerManager) context.getSystemService(Context.POWER_SERVICE);
    
    if (powerManager.isPowerSaveMode()) {
        enablePowerSaveMode(true);
        
        // Reduce connection attempts
        setAutoConnectEnabled(false);
        
        // Implement connection pooling
        implementConnectionPooling();
    }
}
```

## Build Configuration

### Gradle Dependencies

```gradle
dependencies {
    implementation 'androidx.appcompat:appcompat:1.6.1'
    implementation 'androidx.core:core:1.12.0'
    
    // JSON processing
    implementation 'org.json:json:20230227'
    
    // Testing
    testImplementation 'junit:junit:4.13.2'
    testImplementation 'androidx.test:core:1.5.0'
    testImplementation 'org.mockito:mockito-core:5.1.1'
    
    androidTestImplementation 'androidx.test.ext:junit:1.1.5'
    androidTestImplementation 'androidx.test.espresso:espresso-core:3.5.1'
}
```

### ProGuard Configuration

```proguard
# Keep WiFi Aware classes
-keep class pt.iris.network.lib.** { *; }
-keep class android.net.wifi.aware.** { *; }

# Keep callback interfaces
-keepclassmembers class * implements pt.iris.network.lib.WifiAwareLibrary$WifiAwareCallback {
    public void on*(...);
}

# Keep model classes
-keep class pt.iris.network.lib.WifiAwareModels$** { *; }
```

## Testing and Validation

### Unit Tests

```java
@RunWith(AndroidJUnit4.class)
public class WifiAwareLibraryTest {
    
    @Mock
    private Context mockContext;
    
    @Mock
    private WifiAwareManager mockWifiAwareManager;
    
    private WifiAwareLibrary wifiAwareLibrary;
    
    @Before
    public void setUp() {
        MockitoAnnotations.openMocks(this);
        
        when(mockContext.getSystemService(Context.WIFI_AWARE_SERVICE))
            .thenReturn(mockWifiAwareManager);
        
        wifiAwareLibrary = new WifiAwareLibrary(mockContext);
    }
    
    @Test
    public void testWifiAwareSupported() {
        PackageManager mockPackageManager = mock(PackageManager.class);
        when(mockContext.getPackageManager()).thenReturn(mockPackageManager);
        when(mockPackageManager.hasSystemFeature(PackageManager.FEATURE_WIFI_AWARE))
            .thenReturn(true);
        
        assertTrue(wifiAwareLibrary.isWifiAwareSupported());
    }
    
    @Test
    public void testSessionInitialization() {
        when(mockWifiAwareManager.isAvailable()).thenReturn(true);
        
        WifiAwareLibrary.WifiAwareCallback callback = mock(WifiAwareLibrary.WifiAwareCallback.class);
        wifiAwareLibrary.addCallback(callback);
        
        wifiAwareLibrary.initialize();
        
        verify(mockWifiAwareManager).attach(any(), any());
    }
}
```

### Integration Tests

```java
@RunWith(AndroidJUnit4.class)
@LargeTest
public class WifiAwareIntegrationTest {
    
    @Rule
    public ActivityTestRule<TestActivity> activityRule = 
        new ActivityTestRule<>(TestActivity.class);
    
    private WifiAwareLibrary wifiAwareLibrary;
    private CountDownLatch connectionLatch;
    
    @Before
    public void setUp() {
        Context context = activityRule.getActivity();
        wifiAwareLibrary = new WifiAwareLibrary(context);
        connectionLatch = new CountDownLatch(1);
        
        wifiAwareLibrary.addCallback(new WifiAwareLibrary.WifiAwareCallback() {
            @Override
            public void onConnectionEstablished(PeerHandle peerHandle, 
                                              WifiAwareModels.DataPathInfo dataPath) {
                connectionLatch.countDown();
            }
            
            // Implement other callback methods...
        });
    }
    
    @Test
    public void testPeerDiscoveryAndConnection() throws InterruptedException {
        // Test requires two devices with WiFi Aware capability
        assumeTrue("WiFi Aware not supported", wifiAwareLibrary.isWifiAwareSupported());
        assumeTrue("WiFi Aware not available", wifiAwareLibrary.isWifiAwareAvailable());
        
        wifiAwareLibrary.initialize();
        wifiAwareLibrary.startBoth();
        
        // Wait for connection establishment (requires another device)
        boolean connected = connectionLatch.await(30, TimeUnit.SECONDS);
        assertTrue("Connection not established within timeout", connected);
    }
}
```

## Future Enhancements

### Planned Features

- **Enhanced Security**: Additional encryption layers for data paths
- **Quality of Service**: Bandwidth and latency optimization
- **Mesh Routing**: Multi-hop routing through WiFi Aware network
- **Battery Optimization**: Advanced power management strategies
- **Network Analytics**: Detailed network performance analysis

### Research Applications

This WiFi Aware library enables research in:

- **Proximity-based Networking**: Device-to-device communication patterns
- **Mesh Network Optimization**: Self-organizing network topologies
- **Mobile Edge Computing**: Collaborative processing at network edge
- **Privacy-preserving Discovery**: Anonymous device discovery mechanisms
- **Energy-efficient Networking**: Power-aware networking protocols

The library provides a robust foundation for developing advanced wireless networking applications while abstracting the complexity of the underlying WiFi Aware protocol implementation.
