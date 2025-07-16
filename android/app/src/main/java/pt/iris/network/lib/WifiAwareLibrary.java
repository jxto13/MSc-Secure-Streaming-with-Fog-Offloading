package pt.iris.network.lib;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.MacAddress;
import android.net.Network;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.Characteristics;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.IdentityChangedListener;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Build;
import android.util.Log;

import androidx.annotation.NonNull;
import androidx.annotation.RequiresApi;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Main WiFi Aware Library - Orchestrates all WiFi Aware functionality
 * 
 * This library provides a clean, modular interface for WiFi Aware networking:
 * - Discovery session management (publish/subscribe)
 * - Data path connection management
 * - Role negotiation and capability exchange
 * - Resource tracking and cleanup
 */
@SuppressLint("MissingPermission")
public class WifiAwareLibrary implements 
    WifiAwareDiscoveryManager.DiscoveryCallback,
    WifiAwareDataPathManager.DataPathCallback {
    
    private static final String TAG = "WifiAwareLibrary";
    
    // Core components
    private final Context context;
    private final String serviceName;
    private final UUID deviceUUID;
    private final WifiAwareLibraryCallback callback;
    
    // Android managers
    private WifiAwareManager wifiAwareManager;
    private ConnectivityManager connectivityManager;
    private WifiAwareSession wifiAwareSession;
    
    // Our library components
    private WifiAwareDiscoveryManager discoveryManager;
    private WifiAwareDataPathManager dataPathManager;
    
    // State tracking
    private boolean wifiAwareManagerStatus;
    private int maxInterfaces = 0;
    private int maxDataPaths = 0;
    
    // Data structures
    private final Map<PeerHandle, WifiAwareModels.DataPathInfo> activeDataPaths = new ConcurrentHashMap<>();
    private final Map<Network, PeerHandle> networkToPeerMap = new HashMap<>();
    private final Map<PeerHandle, WifiAwareModels.DeviceRole> peerRoles = new ConcurrentHashMap<>();
    private final Map<PeerHandle, WifiAwareModels.DeviceCapabilities> peerCapabilities = new ConcurrentHashMap<>();

    /**
     * Callback interface for library events
     */
    public interface WifiAwareLibraryCallback {
        void onWifiAwareReady();
        void onWifiAwareFailed(String reason);
        void onDataPathEstablished(PeerHandle peerHandle, String ipAddress);
        void onDataPathLost(PeerHandle peerHandle);
        void onDataPathFailed(PeerHandle peerHandle, String reason);
        void addSocketToRust(String ipAddress);
        void removeSocketFromRust(String ipAddress);
    }

    public WifiAwareLibrary(Context context, String serviceName, UUID deviceUUID, WifiAwareLibraryCallback callback) {
        this.context = context;
        this.serviceName = serviceName;
        this.deviceUUID = deviceUUID;
        this.callback = callback;
    }

    /**
     * Initialize the WiFi Aware library
     */
    @RequiresApi(api = Build.VERSION_CODES.O)
    public void initialize() {
        Log.d(TAG, "Initializing WiFi Aware Library with UUID: " + deviceUUID.toString());

        // Get system managers
        this.connectivityManager = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
        this.wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

        // Initialize WiFi Aware manager
        if (initializeWifiAwareManager()) {
            createWifiAwareSession();
        } else {
            callback.onWifiAwareFailed("WiFi Aware not available on this device");
        }
    }

    /**
     * Initialize Wi-Fi Aware Manager with proper data path management
     */
    @RequiresApi(api = Build.VERSION_CODES.O)
    private boolean initializeWifiAwareManager() {
        if (!wifiAwareManager.isAvailable()) {
            wifiAwareManagerStatus = false;
            Log.e(TAG, "WiFi Aware Manager not available on this device");
            return false;
        }

        wifiAwareManagerStatus = true;
        Characteristics characteristics = wifiAwareManager.getCharacteristics();

        Log.d(TAG, "=== WiFi Aware Characteristics ===");
        Log.d(TAG, "Aware Pairing Supported: " + characteristics.isAwarePairingSupported());
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            Log.d(TAG, "Instant Communication Mode Supported: " + characteristics.isInstantCommunicationModeSupported());
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            Log.d(TAG, "Set Channel On Data Path Supported: " + wifiAwareManager.isSetChannelOnDataPathSupported());
        }
        Log.d(TAG, "Number of Supported Data Interfaces: " + characteristics.getNumberOfSupportedDataInterfaces());
        Log.d(TAG, "Number of Supported Data Paths: " + characteristics.getNumberOfSupportedDataPaths());

        // Get current available resources
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            Log.d(TAG, "=== Current Available Resources ===");
            Log.d(TAG, "Available Data Paths Count: " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
            Log.d(TAG, "Available Subscribe Sessions Count: " + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount());
            Log.d(TAG, "Available Publish Sessions Count: " + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());
        }

        maxInterfaces = characteristics.getNumberOfSupportedDataInterfaces();
        maxDataPaths = characteristics.getNumberOfSupportedDataPaths();

        Log.d(TAG, "Max Interfaces: " + maxInterfaces + ", Max Data Paths: " + maxDataPaths);

        return true;
    }

    /**
     * Attach to Wi-Fi Aware and create a session
     */
    @RequiresApi(api = Build.VERSION_CODES.O)
    private void createWifiAwareSession() {
        if (!wifiAwareManagerStatus) return;

        wifiAwareManager.attach(new AttachCallback() {
            @Override
            public void onAttached(WifiAwareSession session) {
                super.onAttached(session);
                wifiAwareSession = session;
                Log.d(TAG, "WiFi Aware session successfully attached");

                // Initialize our managers
                initializeManagers();

                // Start discovery sessions
                discoveryManager.startPublisher(session);
                discoveryManager.startSubscriber(session);

                callback.onWifiAwareReady();
            }

            @Override
            public void onAttachFailed() {
                super.onAttachFailed();
                Log.e(TAG, "WiFi Aware session attachment failed");
                callback.onWifiAwareFailed("Session attachment failed");
            }

            @Override
            public void onAwareSessionTerminated() {
                super.onAwareSessionTerminated();
                Log.w(TAG, "WiFi Aware session terminated, cleaning up");
                cleanupAllDataPaths();
            }
        }, new IdentityChangedListener() {
            @Override
            public void onIdentityChanged(byte[] mac) {
                super.onIdentityChanged(mac);
                Log.d(TAG, "WiFi Aware identity changed");
            }

            @Override
            public void onClusterIdChanged(int clusterEventType, @NonNull MacAddress clusterId) {
                super.onClusterIdChanged(clusterEventType, clusterId);
                Log.d(TAG, "Cluster event " + clusterEventType + " - ID: " + clusterId);
            }
        }, null);
    }

    /**
     * Initialize our internal managers
     */
    private void initializeManagers() {
        // Initialize discovery manager
        discoveryManager = new WifiAwareDiscoveryManager(
                wifiAwareManager,
                serviceName,
                deviceUUID,
                this, // We implement the callback
                peerCapabilities
        );

        // Initialize data path manager
        dataPathManager = new WifiAwareDataPathManager(
                context,
                connectivityManager,
                wifiAwareManager,
                activeDataPaths,
                networkToPeerMap,
                this, // We implement the callback
                maxDataPaths
        );
    }

    // ========== WifiAwareDiscoveryManager.DiscoveryCallback Implementation ==========

    @Override
    public WifiAwareModels.DeviceCapabilities getMyCapabilities() {
        int availableInterfaces = maxInterfaces > 0 ? 
                Math.max(0, maxInterfaces - getCurrentInterfaceUsage()) : 0;
        int availableDataPaths = wifiAwareManagerStatus && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ? 
                wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount() : maxDataPaths;

        return new WifiAwareModels.DeviceCapabilities(availableInterfaces, availableDataPaths, deviceUUID);
    }

    @Override
    public void onCapabilityExchange(PeerHandle peerHandle, WifiAwareModels.DeviceCapabilities peerCapabilities, boolean asPublisher) {
        Log.d(TAG, "Capability exchange with peer " + peerHandle + 
                " - Peer can create connection: " + peerCapabilities.canCreateConnection());

        // Get our capabilities
        WifiAwareModels.DeviceCapabilities myCapabilities = getMyCapabilities();
        Log.d(TAG, "My capabilities - Can create connection: " + myCapabilities.canCreateConnection());

        // Only proceed with connection if both devices can create connections
        if (!myCapabilities.canCreateConnection()) {
            Log.w(TAG, "Cannot create connection - no available data paths on this device");
            return;
        }

        if (!peerCapabilities.canCreateConnection()) {
            Log.w(TAG, "Cannot create connection - peer has no available data paths");
            return;
        }

        // Determine who should initiate the connection
        boolean shouldInitiate = WifiAwareRoleManager.shouldInitiateConnection(myCapabilities, peerCapabilities);

        // Use the appropriate discovery session
        DiscoverySession sessionToUse = asPublisher ? 
                discoveryManager.getPublisherSession() : 
                discoveryManager.getSubscriberSession();

        if (shouldInitiate) {
            Log.d(TAG, "Acting as INITIATOR - will initiate data path with peer " + peerHandle);
            dataPathManager.establishDataPath(peerHandle, sessionToUse, false); // false = initiator
        } else {
            Log.d(TAG, "Acting as RESPONDER - will accept data path from peer " + peerHandle);
            dataPathManager.establishDataPath(peerHandle, sessionToUse, true); // true = responder
        }
    }

    @Override
    public void onServiceLost(PeerHandle peerHandle) {
        // Clean up any existing data path with this peer
        if (activeDataPaths.containsKey(peerHandle)) {
            WifiAwareModels.DataPathInfo dataPathInfo = activeDataPaths.remove(peerHandle);
            if (dataPathInfo != null) {
                networkToPeerMap.remove(dataPathInfo.network);
                removeSocketFromRust(dataPathInfo.socketAddress); // Use socket address for cleanup
                cleanupPeerRole(peerHandle);
                callback.onDataPathLost(peerHandle);
                Log.d(TAG, "Cleaned up data path for lost service: " + peerHandle);
            }
        }
    }

    // ========== WifiAwareDataPathManager.DataPathCallback Implementation ==========

    @Override
    public void onDataPathAvailable(Network network, PeerHandle peerHandle, String ipAddress) {
        Log.d(TAG, "Data path available: " + ipAddress + " for peer: " + peerHandle);
        callback.onDataPathEstablished(peerHandle, ipAddress);
        logDataPathStats();
    }

    @Override
    public void onDataPathLost(Network network, PeerHandle peerHandle) {
        cleanupPeerRole(peerHandle);
        callback.onDataPathLost(peerHandle);
        logDataPathStats();
    }

    @Override
    public void onDataPathFailed(PeerHandle peerHandle, String reason) {
        Log.e(TAG, "Data path failed for peer " + peerHandle + ": " + reason);
        callback.onDataPathFailed(peerHandle, reason);
    }

    @Override
    public void addSocketToRust(String ipAddress) {
        callback.addSocketToRust(ipAddress);
    }

    @Override
    public void removeSocketFromRust(String ipAddress) {
        callback.removeSocketFromRust(ipAddress);
    }

    // ========== Public API Methods ==========

    /**
     * Get current interface usage
     */
    private int getCurrentInterfaceUsage() {
        return (discoveryManager != null && 
                (discoveryManager.isPublisherActive() || discoveryManager.isSubscriberActive())) ? 1 : 0;
    }

    /**
     * Clean up role information for a specific peer
     */
    private void cleanupPeerRole(PeerHandle peerHandle) {
        peerRoles.remove(peerHandle);
        peerCapabilities.remove(peerHandle);
        Log.d(TAG, "Cleaned up role information for peer: " + peerHandle);
    }

    /**
     * Clean up all active data paths
     */
    private void cleanupAllDataPaths() {
        Log.d(TAG, "Cleaning up all " + activeDataPaths.size() + " data paths");

        // Remove all sockets from Rust layer first
        for (WifiAwareModels.DataPathInfo dataPathInfo : activeDataPaths.values()) {
            if (dataPathInfo.isActive) {
                removeSocketFromRust(dataPathInfo.socketAddress); // Use socket address for cleanup
                dataPathInfo.isActive = false;
            }
        }

        // Clear all tracking maps
        activeDataPaths.clear();
        networkToPeerMap.clear();
        peerRoles.clear();
        peerCapabilities.clear();

        Log.d(TAG, "All data paths and role information cleaned up successfully");
    }

    /**
     * Log current data path statistics
     */
    private void logDataPathStats() {
        int activeCount = activeDataPaths.size();
        int availableDataPaths = (wifiAwareManagerStatus && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) ?
                wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount() : 0;

        Log.d(TAG, "=== Data Path Statistics ===");
        Log.d(TAG, "Active data paths: " + activeCount + "/" + maxDataPaths);
        Log.d(TAG, "System available data paths: " + availableDataPaths);

        if (activeCount > 0) {
            for (Map.Entry<PeerHandle, WifiAwareModels.DataPathInfo> entry : activeDataPaths.entrySet()) {
                WifiAwareModels.DataPathInfo info = entry.getValue();
                long ageSeconds = (System.currentTimeMillis() - info.createdTime) / 1000;
                Log.d(TAG, "Peer " + entry.getKey() + " -> Display IPv6: " + info.ipAddress + 
                        ", Socket IPv6: " + info.socketAddress + 
                        ", Active: " + info.isActive + ", Age: " + ageSeconds + "s");
            }
        }

        // Warn if approaching limits
        if (activeCount >= maxDataPaths * 0.8) {
            Log.w(TAG, "Approaching maximum data path limit: " + activeCount + "/" + maxDataPaths);
        }
    }

    /**
     * Get statistics string for UI display
     */
    public String getDataPathStatistics() {
        if (!wifiAwareManagerStatus) {
            return "WiFi Aware not available";
        }

        int activeCount = activeDataPaths.size();
        int availableDataPaths = (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) ?
                wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount() : 0;

        return String.format("Data Paths: %d/%d active, %d available", 
                activeCount, maxDataPaths, availableDataPaths);
    }

    /**
     * Get active connections for UI display
     */
    public List<WifiAwareModels.ConnectionInfo> getActiveConnections() {
        List<WifiAwareModels.ConnectionInfo> connections = new ArrayList<>();

        for (Map.Entry<PeerHandle, WifiAwareModels.DataPathInfo> entry : activeDataPaths.entrySet()) {
            WifiAwareModels.DataPathInfo dataPath = entry.getValue();
            if (dataPath.isActive) {
                String myRole = getMyRoleWithPeer(entry.getKey());
                String peerRole = getPeerRole(entry.getKey());

                // Use socket address (with interface) for UI display
                String displayAddress = dataPath.socketAddress;
                
                WifiAwareModels.ConnectionInfo connectionInfo = new WifiAwareModels.ConnectionInfo(
                        dataPath.peerHandle.toString(),
                        displayAddress, // Now includes interface name
                        dataPath.createdTime,
                        dataPath.isActive,
                        myRole,
                        peerRole
                );
                connections.add(connectionInfo);
            }
        }

        return connections;
    }

    /**
     * Get the role for a specific peer
     */
    private String getPeerRole(PeerHandle peerHandle) {
        WifiAwareModels.DeviceRole role = peerRoles.get(peerHandle);
        return role != null ? role.toString() : "UNKNOWN";
    }

    /**
     * Get my role with a specific peer
     */
    private String getMyRoleWithPeer(PeerHandle peerHandle) {
        WifiAwareModels.DeviceRole peerRole = peerRoles.get(peerHandle);
        if (peerRole == WifiAwareModels.DeviceRole.MASTER) {
            return "SLAVE";
        } else if (peerRole == WifiAwareModels.DeviceRole.SLAVE) {
            return "MASTER";
        }
        return "UNKNOWN";
    }

    /**
     * Check if we can create more data paths
     */
    public boolean canCreateMoreDataPaths() {
        if (!wifiAwareManagerStatus) return false;

        int availableFromSystem = (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) ?
                wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount() : maxDataPaths;
        boolean belowInternalLimit = activeDataPaths.size() < maxDataPaths;

        return availableFromSystem > 0 && belowInternalLimit;
    }

    /**
     * Check if WiFi Aware is active
     */
    public boolean isWifiAwareActive() {
        return wifiAwareManagerStatus && wifiAwareSession != null;
    }

    /**
     * Get active data path count
     */
    public int getActiveDataPathCount() {
        int count = 0;
        for (WifiAwareModels.DataPathInfo dataPath : activeDataPaths.values()) {
            if (dataPath.isActive) {
                count++;
            }
        }
        return count;
    }

    /**
     * Shutdown the library and clean up all resources
     */
    public void shutdown() {
        Log.d(TAG, "Shutting down WiFi Aware Library");

        // Stop discovery sessions
        if (discoveryManager != null) {
            discoveryManager.stopPublisher();
            discoveryManager.stopSubscriber();
        }

        // Clean up all data paths and network callbacks
        cleanupAllDataPaths();
        
        // Clean up data path manager to prevent "not in cache" errors
        if (dataPathManager != null) {
            dataPathManager.cleanup();
        }

        // Close the main WiFi Aware session
        if (wifiAwareSession != null) {
            wifiAwareSession.close();
            wifiAwareSession = null;
            Log.d(TAG, "WiFi Aware session closed");
        }
    }
}
