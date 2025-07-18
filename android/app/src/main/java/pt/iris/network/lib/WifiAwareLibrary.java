package pt.iris.network.lib;

import android.annotation.SuppressLint;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.PackageManager;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.util.Log;

import java.io.IOException;
import java.net.Inet6Address;
import java.net.ServerSocket;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;

import pt.iris.network.lib.WifiAwareModels.DeviceRole;
import pt.iris.network.lib.WifiAwareModels.DataPathInfo;
import pt.iris.network.lib.WifiAwareModels.SessionInfo;
import pt.iris.network.lib.WifiAwareModels.SessionState;

/**
 * Complete Wi-Fi Aware implementation for Android
 * Handles discovery, publishing, messaging, and data path connections
 */
@SuppressLint("MissingPermission")
public class WifiAwareLibrary {
    private static final String TAG = "WifiAwareLibrary";
    public static final String SERVICE_NAME = "Iris_Streaming_Service";
    
    private final Context context;
    private final WifiAwareManager wifiAwareManager;
    private final ConnectivityManager connectivityManager;
    
    // Session management
    private WifiAwareSession wifiAwareSession;
    private PublishDiscoverySession publishSession;
    private SubscribeDiscoverySession subscribeSession;
    
    // Connection tracking
    private final Map<PeerHandle, DataPathInfo> activeConnections = new ConcurrentHashMap<>();
    private final Map<PeerHandle, DataPathInfo> discoveredPeers = new ConcurrentHashMap<>();
    private final List<WifiAwareCallback> callbacks = new CopyOnWriteArrayList<>();
    private final List<String> connectionLogs = new CopyOnWriteArrayList<>();
    
    // Network management
    private ServerSocket serverSocket;
    private int serverPort = 0;
    private Network currentNetwork;
    private ConnectivityManager.NetworkCallback networkCallback;
    
    // State
    private boolean isPublishing = false;
    private boolean isSubscribing = false;
    private final UUID deviceUUID = UUID.randomUUID();
    private DeviceRole currentRole = DeviceRole.UNKNOWN;

    public interface WifiAwareCallback {
        void onWifiAwareAvailable(boolean available);
        void onSessionAttached(WifiAwareSession session);
        void onPublishStarted(PublishDiscoverySession session);
        void onSubscribeStarted(SubscribeDiscoverySession session);
        void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo);
        void onMessageReceived(PeerHandle peerHandle, byte[] message);
        void onConnectionEstablished(PeerHandle peerHandle, DataPathInfo dataPath);
        void onConnectionLost(PeerHandle peerHandle);
        void onPeerDiscovered(PeerHandle peerHandle);
        void onPeerLost(PeerHandle peerHandle);
        void onRoleChanged(DeviceRole newRole);
        void onError(String error);
    }
    
    private final BroadcastReceiver wifiAwareStateReceiver = new BroadcastReceiver() {
        @Override
        public void onReceive(Context context, Intent intent) {
            if (WifiAwareManager.ACTION_WIFI_AWARE_STATE_CHANGED.equals(intent.getAction())) {
                boolean isAvailable = wifiAwareManager != null && wifiAwareManager.isAvailable();
                Log.d(TAG, "Wi-Fi Aware state changed: " + isAvailable);
                
                for (WifiAwareCallback callback : callbacks) {
                    callback.onWifiAwareAvailable(isAvailable);
                }
                
                if (!isAvailable) {
                    // Clean up all sessions when Wi-Fi Aware becomes unavailable
                    cleanup();
                }
            }
        }
    };

    public WifiAwareLibrary(Context context) {
        this.context = context.getApplicationContext();
        this.wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);
        this.connectivityManager = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);

        // Register for Wi-Fi Aware state changes
        IntentFilter filter = new IntentFilter(WifiAwareManager.ACTION_WIFI_AWARE_STATE_CHANGED);
        context.registerReceiver(wifiAwareStateReceiver, filter);
    }
    
    public boolean isWifiAwareSupported() {
        return context.getPackageManager().hasSystemFeature(PackageManager.FEATURE_WIFI_AWARE);
    }
    
    public boolean isWifiAwareAvailable() {
        return wifiAwareManager != null && wifiAwareManager.isAvailable();
    }
    
    public void addCallback(WifiAwareCallback callback) {
        callbacks.add(callback);
    }
    
    public void removeCallback(WifiAwareCallback callback) {
        callbacks.remove(callback);
    }
    
    public void initialize() {
        if (!isWifiAwareSupported()) {
            notifyError("Wi-Fi Aware not supported on this device");
            return;
        }
        
        if (!isWifiAwareAvailable()) {
            notifyError("Wi-Fi Aware not currently available");
            return;
        }
        
        attachToWifiAware();
    }
    
    private void attachToWifiAware() {
        if (wifiAwareSession != null) {
            Log.d(TAG, "Already attached to Wi-Fi Aware");
            return;
        }
        
        wifiAwareManager.attach(new AttachCallback() {
            @Override
            public void onAttached(WifiAwareSession session) {
                Log.d(TAG, "Successfully attached to Wi-Fi Aware");
                addConnectionLog("Wi-Fi Aware session attached successfully");
                wifiAwareSession = session;
                
                for (WifiAwareCallback callback : callbacks) {
                    callback.onSessionAttached(session);
                }
            }
            
            @Override
            public void onAttachFailed() {
                Log.e(TAG, "Failed to attach to Wi-Fi Aware");
                addConnectionLog("Failed to attach to Wi-Fi Aware session");
                notifyError("Failed to attach to Wi-Fi Aware");
            }
        }, null);
    }
    
    public void startPublishing() {
        if (wifiAwareSession == null) {
            notifyError("Wi-Fi Aware session not available");
            return;
        }
        
        if (isPublishing) {
            Log.d(TAG, "Already publishing");
            return;
        }
        
        PublishConfig config = new PublishConfig.Builder()
                .setServiceName(SERVICE_NAME)
                .setServiceSpecificInfo(deviceUUID.toString().getBytes())
                .build();
        
        wifiAwareSession.publish(config, new DiscoverySessionCallback() {
            @Override
            public void onPublishStarted(PublishDiscoverySession session) {
                Log.d(TAG, "Publishing started");
                publishSession = session;
                isPublishing = true;
                
                // Only set role to MASTER if we're not already subscribing
                // If we're doing both, role will be determined by who discovers whom first
                if (!isSubscribing) {
                    updateRole(DeviceRole.MASTER);
                }
                
                for (WifiAwareCallback callback : callbacks) {
                    callback.onPublishStarted(session);
                }
            }
            
            @Override
            public void onSessionConfigFailed() {
                Log.e(TAG, "Failed to start publishing");
                notifyError("Failed to start publishing");
            }
            
            @Override
            public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
                Log.d(TAG, "Message received from: " + peerHandle);
                handleMessageReceived(peerHandle, message);
            }
            
            @Override
            public void onServiceLost(PeerHandle peerHandle, int reason) {
                Log.d(TAG, "Service lost from peer: " + peerHandle + ", reason: " + reason);
                handlePeerLost(peerHandle);
            }
            
            @Override
            public void onSessionTerminated() {
                Log.d(TAG, "Publish session terminated");
                publishSession = null;
                isPublishing = false;
                handleSessionTermination();
            }
        }, null);
    }
    
    public void startSubscribing() {
        if (wifiAwareSession == null) {
            notifyError("Wi-Fi Aware session not available");
            return;
        }
        
        if (isSubscribing) {
            Log.d(TAG, "Already subscribing");
            return;
        }
        
        SubscribeConfig config = new SubscribeConfig.Builder()
                .setServiceName(SERVICE_NAME)
                .build();
        
        wifiAwareSession.subscribe(config, new DiscoverySessionCallback() {
            SubscribeDiscoverySession session = null;
            @Override
            public void onSubscribeStarted(SubscribeDiscoverySession session) {
                Log.d(TAG, "Subscribing started");
                subscribeSession = session;
                isSubscribing = true;
                this.session = session;
                
                for (WifiAwareCallback callback : callbacks) {
                    callback.onSubscribeStarted(session);
                }
            }
            
            @Override
            public void onSessionConfigFailed() {
                Log.e(TAG, "Failed to start subscribing");
                notifyError("Failed to start subscribing");
            }
            
            @Override
            public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
                Log.d(TAG, "Service discovered from peer: " + peerHandle);
                addConnectionLog("Service discovered from peer: " + peerHandle.toString().substring(0, Math.min(8, peerHandle.toString().length())));
                
                // Don't immediately set role to SLAVE here - wait for message exchange
                // Role will be determined based on who initiates the connection
                
                String peerName = new String(serviceSpecificInfo);
                DataPathInfo pathInfo = new DataPathInfo(peerHandle);
                discoveredPeers.put(peerHandle, pathInfo);
                notifyDiscoveredPeersChanged();
                
                // Notify callbacks about peer discovery
                for (WifiAwareCallback callback : callbacks) {
                    callback.onPeerDiscovered(peerHandle);
                }

                // Send hello message to establish communication
                String message = "HELLO_FROM_" + getDeviceId();
                session.sendMessage(peerHandle, 0, message.getBytes());
                
                // We are initiating contact, so we act as the CLIENT/SLAVE
                updateRole(DeviceRole.SLAVE);
                Log.d(TAG, "Role set to SLAVE - we initiated contact with discovered service");
                addConnectionLog("Role set to SLAVE - initiated contact with service");
            }
            
            @Override
            public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
                Log.d(TAG, "Message received from: " + peerHandle);
                handleMessageReceived(peerHandle, message);
            }
            
            @Override
            public void onServiceLost(PeerHandle peerHandle, int reason) {
                Log.d(TAG, "Service lost from peer: " + peerHandle + ", reason: " + reason);
                handlePeerLost(peerHandle);
            }
            
            @Override
            public void onSessionTerminated() {
                Log.d(TAG, "Subscribe session terminated");
                subscribeSession = null;
                isSubscribing = false;
                handleSessionTermination();
            }
        }, null);
    }
    
    public void sendMessage(PeerHandle peerHandle, byte[] message) {
        DiscoverySession session = isPublishing ? publishSession : subscribeSession;
        if (session != null) {
            session.sendMessage(peerHandle, 1, message);
        } else {
            notifyError("No active discovery session to send message");
        }
    }
    
    private void handleMessageReceived(PeerHandle peerHandle, byte[] message) {
        String messageStr = new String(message);
        Log.d(TAG, "Received message: " + messageStr + " from " + peerHandle);
        
        // Role determination: If we receive a HELLO_FROM_, it means someone discovered us
        // This makes us the master/server since they're connecting to our published service
        if (messageStr.startsWith("HELLO_FROM_")) {
            // We are receiving a connection request, so we act as the SERVER/MASTER
            updateRole(DeviceRole.MASTER);
            Log.d(TAG, "Role set to MASTER - received connection request from peer");
            addConnectionLog("Role set to MASTER - received connection request");
            
            // Add peer to discovered list if not already present
            if (!discoveredPeers.containsKey(peerHandle)) {
                DataPathInfo pathInfo = new DataPathInfo(peerHandle);
                discoveredPeers.put(peerHandle, pathInfo);
                notifyDiscoveredPeersChanged();
                addConnectionLog("Added peer to discovered list: " + peerHandle.toString().substring(0, Math.min(8, peerHandle.toString().length())));
                
                // Notify callbacks about peer discovery
                for (WifiAwareCallback callback : callbacks) {
                    callback.onPeerDiscovered(peerHandle);
                }
            }
        }
        
        for (WifiAwareCallback callback : callbacks) {
            callback.onMessageReceived(peerHandle, message);
        }
        
        // If this is a connection request, establish data path
        if (messageStr.startsWith("HELLO_FROM_") || "CONNECT_REQUEST".equals(messageStr)) {
            establishDataPath(peerHandle, true);
        }
    }
    
    private void handlePeerLost(PeerHandle peerHandle) {
        Log.d(TAG, "Handling peer lost: " + peerHandle);
        
        // Remove peer from discovered peers and active connections
        DataPathInfo removedPeer = discoveredPeers.remove(peerHandle);
        DataPathInfo removedConnection = activeConnections.remove(peerHandle);
        
        // Notify callbacks about peer/connection loss
        for (WifiAwareCallback callback : callbacks) {
            if (removedPeer != null) {
                callback.onPeerLost(peerHandle);
            }
            if (removedConnection != null) {
                callback.onConnectionLost(peerHandle);
            }
        }
        
        // Update UI
        notifyDiscoveredPeersChanged();
        notifyActiveConnectionsChanged();
        
        // Check if we should reset role when no peers are left
        if (discoveredPeers.isEmpty() && activeConnections.isEmpty()) {
            currentRole = DeviceRole.UNKNOWN;
            Log.d(TAG, "Reset role to UNKNOWN - no peers remaining");
        }
    }
    
    public void establishDataPath(PeerHandle peerHandle, boolean isInitiator) {
        Log.d(TAG, "Attempting to establish data path with peer: " + peerHandle + " (isInitiator: " + isInitiator + ")");
        
        try {
            if (isInitiator) {
                // Create server socket for incoming connections
                if (serverSocket == null) {
                    serverSocket = new ServerSocket(0);
                    serverPort = serverSocket.getLocalPort();
                    Log.d(TAG, "Server socket created on port: " + serverPort);
                }
                
                // Request network as responder (server)
                DiscoverySession currentSession = isPublishing ? publishSession : subscribeSession;
                if (currentSession == null) {
                    String errorLog = "No active discovery session available for network specifier";
                    Log.e(TAG, errorLog);
                    notifyError(errorLog);
                    return;
                }
                
                Log.d(TAG, "Creating WiFi Aware network specifier with session: " + currentSession.getClass().getSimpleName());
                
                WifiAwareNetworkSpecifier.Builder specBuilder = new WifiAwareNetworkSpecifier.Builder(
                        currentSession, peerHandle);
                specBuilder.setPort(serverPort);
                specBuilder.setPskPassphrase("IrisWifiAware2024");
                
                WifiAwareNetworkSpecifier networkSpec;
                try {
                    networkSpec = specBuilder.build();
                    Log.d(TAG, "Network specifier built successfully");
                } catch (Exception e) {
                    String specError = "Failed to build network specifier: " + e.getMessage();
                    Log.e(TAG, specError);
                    notifyError(specError);
                    return;
                }
                
                NetworkRequest request = new NetworkRequest.Builder()
                        .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
                        .setNetworkSpecifier(networkSpec)
                        .build();
                
                Log.d(TAG, "Requesting WiFi Aware network...");
                
                networkCallback = new ConnectivityManager.NetworkCallback() {
                    @Override
                    public void onAvailable(Network network) {
                        Log.d(TAG, "Wi-Fi Aware network available for peer: " + peerHandle);
                        currentNetwork = network;
                    }
                    
                    @Override
                    public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
                        WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();
                        if (peerAwareInfo != null) {
                            Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
                            int peerPort = peerAwareInfo.getPort();
                            
                            Log.d(TAG, "Peer connection info - IP: " + peerIpv6 + ", Port: " + peerPort);
                            
                            DataPathInfo dataPath = new DataPathInfo(
                                    peerHandle, network, peerIpv6.getHostAddress());
                            
                            // Move from discovered to connected
                            discoveredPeers.remove(peerHandle);
                            activeConnections.put(peerHandle, dataPath);
                            
                            Log.d(TAG, "Data path established successfully with peer: " + peerHandle);
                            
                            for (WifiAwareCallback callback : callbacks) {
                                callback.onConnectionEstablished(peerHandle, dataPath);
                            }
                        }
                    }
                    
                    @Override
                    public void onLost(Network network) {
                        Log.d(TAG, "Wi-Fi Aware network lost for peer: " + peerHandle);
                        currentNetwork = null;
                        
                        DataPathInfo removedPath = activeConnections.remove(peerHandle);
                        if (removedPath != null) {
                            removedPath.isActive = false;
                            
                            for (WifiAwareCallback callback : callbacks) {
                                callback.onConnectionLost(peerHandle);
                            }
                        }
                    }
                    
                    @Override
                    public void onUnavailable() {
                        Log.e(TAG, "Wi-Fi Aware network unavailable for peer: " + peerHandle);
                        notifyError("Wi-Fi Aware network unavailable for peer: " + peerHandle);
                    }
                };
                
                connectivityManager.requestNetwork(request, networkCallback);
                
            } else {
                // Request network as initiator (client)
                DiscoverySession currentSession = isPublishing ? publishSession : subscribeSession;
                if (currentSession == null) {
                    String errorLog = "No active discovery session available for client network specifier";
                    Log.e(TAG, errorLog);
                    notifyError(errorLog);
                    return;
                }
                
                Log.d(TAG, "Creating WiFi Aware client network specifier with session: " + currentSession.getClass().getSimpleName());
                
                WifiAwareNetworkSpecifier.Builder specBuilder = new WifiAwareNetworkSpecifier.Builder(
                        currentSession, peerHandle);
                specBuilder.setPskPassphrase("IrisWifiAware2024");
                
                WifiAwareNetworkSpecifier networkSpec;
                try {
                    networkSpec = specBuilder.build();
                    Log.d(TAG, "Client network specifier built successfully");
                } catch (Exception e) {
                    String specError = "Failed to build client network specifier: " + e.getMessage();
                    Log.e(TAG, specError);
                    notifyError(specError);
                    return;
                }
                
                NetworkRequest request = new NetworkRequest.Builder()
                        .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
                        .setNetworkSpecifier(networkSpec)
                        .build();
                
                Log.d(TAG, "Requesting WiFi Aware client network...");
                
                networkCallback = new ConnectivityManager.NetworkCallback() {
                    @Override
                    public void onAvailable(Network network) {
                        Log.d(TAG, "Wi-Fi Aware client network available for peer: " + peerHandle);
                        currentNetwork = network;
                    }
                    
                    @Override
                    public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
                        WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();
                        if (peerAwareInfo != null) {
                            Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
                            int peerPort = peerAwareInfo.getPort();
                            
                            Log.d(TAG, "Server connection info - IP: " + peerIpv6 + ", Port: " + peerPort);
                            
                            DataPathInfo dataPath = new DataPathInfo(
                                    peerHandle, network, peerIpv6.getHostAddress());
                            
                            // Move from discovered to connected
                            discoveredPeers.remove(peerHandle);
                            activeConnections.put(peerHandle, dataPath);
                            
                            Log.d(TAG, "Client data path established successfully with peer: " + peerHandle);
                            
                            for (WifiAwareCallback callback : callbacks) {
                                callback.onConnectionEstablished(peerHandle, dataPath);
                            }
                        }
                    }
                    
                    @Override
                    public void onLost(Network network) {
                        Log.d(TAG, "Wi-Fi Aware client network lost for peer: " + peerHandle);
                        currentNetwork = null;
                        
                        DataPathInfo removedPath = activeConnections.remove(peerHandle);
                        if (removedPath != null) {
                            removedPath.isActive = false;
                            
                            for (WifiAwareCallback callback : callbacks) {
                                callback.onConnectionLost(peerHandle);
                            }
                        }
                    }
                    
                    @Override
                    public void onUnavailable() {
                        Log.e(TAG, "Wi-Fi Aware client network unavailable for peer: " + peerHandle);
                        notifyError("Wi-Fi Aware client network unavailable for peer: " + peerHandle);
                    }
                };
                
                connectivityManager.requestNetwork(request, networkCallback);
            }
            
        } catch (IOException e) {
            String errorLog = "Failed to establish data path: " + e.getMessage();
            Log.e(TAG, errorLog, e);
            notifyError(errorLog);
        } catch (Exception e) {
            String errorLog = "Unexpected error during data path establishment: " + e.getMessage();
            Log.e(TAG, errorLog, e);
            notifyError(errorLog);
        }
    }
    
    public Map<PeerHandle, DataPathInfo> getActiveConnections() {
        return new ConcurrentHashMap<>(activeConnections);
    }
    
    public Map<PeerHandle, DataPathInfo> getDiscoveredPeers() {
        return new ConcurrentHashMap<>(discoveredPeers);
    }
    
    public Map<PeerHandle, DataPathInfo> getAllPeers() {
        Map<PeerHandle, DataPathInfo> allPeers = new ConcurrentHashMap<>();
        allPeers.putAll(discoveredPeers);
        allPeers.putAll(activeConnections);
        return allPeers;
    }
    
    public DeviceRole getCurrentRole() {
        return currentRole;
    }
    
    public SessionInfo getSessionInfo() {
        SessionState sessionState = SessionState.IDLE;
        if (isPublishing && isSubscribing) {
            sessionState = SessionState.ACTIVE;
        } else if (isPublishing || isSubscribing) {
            sessionState = SessionState.DISCOVERING;
        }
        
        return new SessionInfo(
            currentRole, 
            sessionState,
            isPublishing, 
            isSubscribing,
            discoveredPeers.size(),
            activeConnections.size()
        );
    }
    
    private void updateRole(DeviceRole newRole) {
        if (currentRole != newRole) {
            Log.d(TAG, "Role changed from " + currentRole + " to " + newRole);
            currentRole = newRole;
            
            for (WifiAwareCallback callback : callbacks) {
                callback.onRoleChanged(newRole);
            }
        }
    }
    
    public String getWifiAwareStats() {
        if (activeConnections.isEmpty()) {
            return "N/A";
        }
        
        int activePaths = 0;
        for (DataPathInfo path : activeConnections.values()) {
            if (path.isActive) {
                activePaths++;
            }
        }
        
        return activePaths + "/" + activeConnections.size();
    }
    
    private String getDeviceId() {
        return deviceUUID.toString().substring(0, 8);
    }
    
    private void notifyDiscoveredPeersChanged() {
        // This method can be extended to notify UI components about peer changes
        // For now, we rely on the callback system
        Log.d(TAG, "Discovered peers changed. Total: " + discoveredPeers.size());
    }
    
    private void notifyActiveConnectionsChanged() {
        // This method can be extended to notify UI components about connection changes
        // For now, we rely on the callback system
        Log.d(TAG, "Active connections changed. Total: " + activeConnections.size());
    }
    
    private void notifyError(String error) {
        Log.e(TAG, error);
        for (WifiAwareCallback callback : callbacks) {
            callback.onError(error);
        }
    }
    
    private void handleSessionTermination() {
        Log.d(TAG, "Handling session termination - cleaning up peers");
        
        // Notify about lost peers
        for (PeerHandle peerHandle : discoveredPeers.keySet()) {
            for (WifiAwareCallback callback : callbacks) {
                callback.onPeerLost(peerHandle);
            }
        }
        
        for (PeerHandle peerHandle : activeConnections.keySet()) {
            for (WifiAwareCallback callback : callbacks) {
                callback.onConnectionLost(peerHandle);
            }
        }
        
        // Clear peer lists
        discoveredPeers.clear();
        activeConnections.clear();
        
        // Reset role when all sessions end
        if (!isPublishing && !isSubscribing) {
            currentRole = DeviceRole.UNKNOWN;
        }
    }
    
    public void cleanup() {
        Log.d(TAG, "Cleaning up Wi-Fi Aware resources");
        
        // Close discovery sessions
        if (publishSession != null) {
            publishSession.close();
            publishSession = null;
            isPublishing = false;
        }
        
        if (subscribeSession != null) {
            subscribeSession.close();
            subscribeSession = null;
            isSubscribing = false;
        }
        
        // Close Wi-Fi Aware session
        if (wifiAwareSession != null) {
            wifiAwareSession.close();
            wifiAwareSession = null;
        }
        
        // Clean up network callback
        if (networkCallback != null && connectivityManager != null) {
            try {
                connectivityManager.unregisterNetworkCallback(networkCallback);
            } catch (Exception e) {
                Log.w(TAG, "Error unregistering network callback", e);
            }
            networkCallback = null;
        }
        
        // Close server socket
        if (serverSocket != null) {
            try {
                serverSocket.close();
            } catch (IOException e) {
                Log.w(TAG, "Error closing server socket", e);
            }
            serverSocket = null;
            serverPort = 0;
        }
        
        // Mark all connections as inactive
        for (DataPathInfo path : activeConnections.values()) {
            path.isActive = false;
        }
        activeConnections.clear();
        discoveredPeers.clear();
        
        // Reset role
        currentRole = DeviceRole.UNKNOWN;
        
        currentNetwork = null;
    }
    
    public void destroy() {
        cleanup();
        
        // Unregister broadcast receiver
        try {
            context.unregisterReceiver(wifiAwareStateReceiver);
        } catch (Exception e) {
            Log.w(TAG, "Error unregistering broadcast receiver", e);
        }
        
        callbacks.clear();
    }
    
    // Getters
    public boolean isPublishing() { return isPublishing; }
    public boolean isSubscribing() { return isSubscribing; }
    public UUID getDeviceUUID() { return deviceUUID; }
    public Network getCurrentNetwork() { return currentNetwork; }
    public int getServerPort() { return serverPort; }

    public DeviceRole getRole() { return currentRole; }
    
    public void clearConnectionLogs() {
        connectionLogs.clear();
    }
    
    public List<String> getConnectionLogs() {
        return new ArrayList<>(connectionLogs);
    }
    
    private void addConnectionLog(String message) {
        String timestamp = new java.text.SimpleDateFormat("HH:mm:ss.SSS").format(new java.util.Date());
        String logEntry = "[" + timestamp + "] " + message;
        connectionLogs.add(logEntry);
        
        // Keep only the last 100 logs to prevent memory issues
        if (connectionLogs.size() > 100) {
            connectionLogs.remove(0);
        }
        
        Log.d(TAG, logEntry);
    }
}