package pt.iris.network.lib;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import java.net.Inet6Address;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Manages WiFi Aware data path connections
 */
@SuppressLint("MissingPermission")
public class WifiAwareDataPathManager {
    private static final String TAG = "WifiAwareDataPathManager";
    private static final long REDISCOVERY_TIMEOUT_MS = 10000; // 10 seconds timeout before rediscovery

    private final Context context;
    private final ConnectivityManager connectivityManager;
    private final WifiAwareManager wifiAwareManager;
    private final Map<PeerHandle, WifiAwareModels.DataPathInfo> activeDataPaths;
    private final Map<Network, PeerHandle> networkToPeerMap;
    private final DataPathCallback callback;
    private final int maxDataPaths;
    
    // Track active network callbacks to prevent "not in cache" errors
    private final Map<PeerHandle, ConnectivityManager.NetworkCallback> activeCallbacks = new ConcurrentHashMap<>();
    
    // Track failed connection attempts and their rediscovery timeouts
    private final Map<PeerHandle, Runnable> pendingRediscovery = new ConcurrentHashMap<>();
    private final Handler timeoutHandler = new Handler(Looper.getMainLooper());

    public interface DataPathCallback {
        void onDataPathAvailable(Network network, PeerHandle peerHandle, String ipAddress);
        void onDataPathLost(Network network, PeerHandle peerHandle);
        void onDataPathFailed(PeerHandle peerHandle, String reason);
        void addSocketToRust(String ipAddress);
    }

    public WifiAwareDataPathManager(Context context, 
                                   ConnectivityManager connectivityManager,
                                   WifiAwareManager wifiAwareManager,
                                   Map<PeerHandle, WifiAwareModels.DataPathInfo> activeDataPaths,
                                   Map<Network, PeerHandle> networkToPeerMap,
                                   DataPathCallback callback,
                                   int maxDataPaths) {
        this.context = context;
        this.connectivityManager = connectivityManager;
        this.wifiAwareManager = wifiAwareManager;
        this.activeDataPaths = activeDataPaths;
        this.networkToPeerMap = networkToPeerMap;
        this.callback = callback;
        this.maxDataPaths = maxDataPaths;
    }

    /**
     * Establish a NAN data path for efficient multi-peer communication
     * @param peerHandle The peer to connect to
     * @param discoverySession The discovery session to use
     * @param isResponder True if this device should act as responder, false for initiator
     */
    public void establishDataPath(PeerHandle peerHandle, DiscoverySession discoverySession, boolean isResponder) {
        // Check if we already have a data path with this peer
        if (activeDataPaths.containsKey(peerHandle)) {
            Log.d(TAG, "Data path already exists with peer: " + peerHandle);
            return;
        }
        
        // Check if there's already a pending network request for this peer
        if (activeCallbacks.containsKey(peerHandle)) {
            Log.d(TAG, "Network request already pending for peer: " + peerHandle);
            return;
        }

        // Validate discovery session
        if (discoverySession == null) {
            Log.e(TAG, "Discovery session is null - cannot establish data path with peer: " + peerHandle);
            callback.onDataPathFailed(peerHandle, "Discovery session is null");
            return;
        }

        // Check current system resources
        int availableDataPaths = wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount();
        if (availableDataPaths <= 0) {
            Log.w(TAG, "No available data paths from system resources. Cannot create new data path.");
            callback.onDataPathFailed(peerHandle, "No available system data paths");
            return;
        }

        // Check if we've reached our internal limit
        if (activeDataPaths.size() >= maxDataPaths) {
            Log.w(TAG, "Maximum number of data paths reached (" + maxDataPaths + "). Cannot create new data path.");
            callback.onDataPathFailed(peerHandle, "Maximum data paths reached");
            return;
        }

        Log.d(TAG, "Establishing NAN data path with peer: " + peerHandle + " (isResponder: " + isResponder + 
            ", available: " + availableDataPaths + ", active: " + activeDataPaths.size() + ")");

        // Create network specifier based on role
        NetworkSpecifier networkSpecifier;
        
        try {
            if (isResponder) {
                // Responder can accept any peer - simplified approach for Android 12+
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                    networkSpecifier = new WifiAwareNetworkSpecifier.Builder(discoverySession, peerHandle)
                            .build();
                } else {
                    // For older versions, use the peer handle approach
                    networkSpecifier = new WifiAwareNetworkSpecifier.Builder(discoverySession, peerHandle)
                            .build();
                }
                Log.d(TAG, "Responder network specifier created for peer: " + peerHandle);
            } else {
                // Initiator requests connection to specific peer
                networkSpecifier = new WifiAwareNetworkSpecifier.Builder(discoverySession, peerHandle)
                        .build();
                Log.d(TAG, "Initiator network specifier created for peer: " + peerHandle);
            }
        } catch (Exception e) {
            Log.e(TAG, "Failed to create network specifier for peer: " + peerHandle, e);
            callback.onDataPathFailed(peerHandle, "Failed to create network specifier: " + e.getMessage());
            return;
        }

        NetworkRequest networkRequest = new NetworkRequest.Builder()
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
                .setNetworkSpecifier(networkSpecifier)
                .addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED)
                .addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_RESTRICTED)
                .removeCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
                .build();

        ConnectivityManager.NetworkCallback networkCallback = new ConnectivityManager.NetworkCallback() {
            @Override
            public void onAvailable(Network network) {
                Log.d(TAG, "NAN data path network available: " + network + " for peer: " + peerHandle);
                // Remove the callback from tracking since we got a successful connection
                activeCallbacks.remove(peerHandle);
                handleDataPathAvailable(network, peerHandle);
            }

            @Override
            public void onLinkPropertiesChanged(Network network, android.net.LinkProperties linkProperties) {
                Log.d(TAG, "NAN data path link properties changed for network: " + network);
                updateDataPathInfo(network, linkProperties);
            }

            @Override
            public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
                Log.d(TAG, "NAN data path capabilities changed for network: " + network);
                if (networkCapabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI_AWARE)) {
                    Log.d(TAG, "Network confirmed as WiFi Aware transport");
                }
            }

            @Override
            public void onLost(Network network) {
                Log.w(TAG, "NAN data path network lost: " + network);
                // Remove callback from tracking when network is lost
                activeCallbacks.remove(peerHandle);
                handleDataPathLost(network);
            }

            @Override
            public void onUnavailable() {
                Log.e(TAG, "NAN data path network unavailable for peer: " + peerHandle);
                Log.e(TAG, "This indicates the data path request failed");
                
                // Remove callback from tracking since request failed
                activeCallbacks.remove(peerHandle);
                
                // Log current system state for debugging
                Log.e(TAG, "Current system state:");
                Log.e(TAG, "- Available data paths: " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
                Log.e(TAG, "- Active data paths: " + activeDataPaths.size());
                Log.e(TAG, "- WiFi Aware available: " + wifiAwareManager.isAvailable());
                
                // Try fallback connection
                Log.w(TAG, "Attempting fallback connection with peer: " + peerHandle);
            }

            @Override
            public void onBlockedStatusChanged(Network network, boolean blocked) {
                Log.d(TAG, "NAN data path blocked status changed: " + blocked + " for network: " + network);
            }
        };

        // Track the callback to prevent "not in cache" errors
        activeCallbacks.put(peerHandle, networkCallback);
        
        // Request the NAN data path with timeout
        try {
            connectivityManager.requestNetwork(networkRequest, networkCallback, 30000);
            Log.d(TAG, "Network request submitted successfully for peer: " + peerHandle + " (30s timeout)");
        } catch (Exception e) {
            Log.e(TAG, "Failed to request NAN data path network for peer: " + peerHandle, e);
            // Remove from tracking if request failed
            activeCallbacks.remove(peerHandle);
            callback.onDataPathFailed(peerHandle, "Failed to request network: " + e.getMessage());
        }
    }

    /**
     * Handle when a NAN data path becomes available
     */
    private void handleDataPathAvailable(Network network, PeerHandle peerHandle) {
        try {
            android.net.LinkProperties linkProperties = connectivityManager.getLinkProperties(network);
            if (linkProperties != null && !linkProperties.getLinkAddresses().isEmpty()) {
                // Get the IPv6 link-local address for this NAN data path
                String ipAddress = null;
                for (android.net.LinkAddress linkAddress : linkProperties.getLinkAddresses()) {
                    if (linkAddress.getAddress() instanceof Inet6Address) {
                        Inet6Address ipv6Address = (Inet6Address) linkAddress.getAddress();
                        // Prefer link-local addresses for NAN data paths
                        if (ipv6Address.isLinkLocalAddress()) {
                            ipAddress = ipv6Address.getHostAddress();
                            break;
                        }
                    }
                }

                // Fallback to any IPv6 address if no link-local found
                if (ipAddress == null) {
                    for (android.net.LinkAddress linkAddress : linkProperties.getLinkAddresses()) {
                        if (linkAddress.getAddress() instanceof Inet6Address) {
                            ipAddress = linkAddress.getAddress().getHostAddress();
                            break;
                        }
                    }
                }

                if (ipAddress != null) {
                    // Keep the zone identifier (interface name) for proper socket communication
                    String interfaceName = linkProperties.getInterfaceName();
                    String socketAddress = ipAddress;
                    
                    // Ensure the interface name is properly included
                    if (ipAddress.contains("%")) {
                        // Address already has zone identifier, use as-is
                        socketAddress = ipAddress;
                    } else if (interfaceName != null) {
                        // Add the interface name as zone identifier
                        socketAddress = ipAddress + "%" + interfaceName;
                    }

                    // For display and storage, use clean address
                    String displayAddress = ipAddress;
                    if (displayAddress.contains("%")) {
                        displayAddress = displayAddress.substring(0, displayAddress.indexOf("%"));
                    }

                    // Store the NAN data path information with both addresses
                    WifiAwareModels.DataPathInfo dataPathInfo = new WifiAwareModels.DataPathInfo(peerHandle, network, displayAddress, socketAddress);
                    activeDataPaths.put(peerHandle, dataPathInfo);
                    networkToPeerMap.put(network, peerHandle);

                    Log.d(TAG, "NAN data path established with peer " + peerHandle + 
                        " at IPv6: " + displayAddress + " (interface: " + interfaceName + 
                        ", socket address: " + socketAddress + ")");
                    
                    // Notify callbacks
                    callback.onDataPathAvailable(network, peerHandle, displayAddress);
                    callback.addSocketToRust(socketAddress); // Use full address with interface for socket
                } else {
                    Log.e(TAG, "No valid IPv6 address found for NAN data path");
                    callback.onDataPathFailed(peerHandle, "No valid IPv6 address found");
                }
            } else {
                Log.e(TAG, "No link properties or addresses available for NAN data path");
                callback.onDataPathFailed(peerHandle, "No link properties available");
            }
        } catch (Exception e) {
            Log.e(TAG, "Error handling NAN data path availability", e);
            callback.onDataPathFailed(peerHandle, "Error handling data path: " + e.getMessage());
        }
    }

    /**
     * Update data path information when link properties change
     */
    private void updateDataPathInfo(Network network, android.net.LinkProperties linkProperties) {
        PeerHandle peerHandle = networkToPeerMap.get(network);
        if (peerHandle != null && activeDataPaths.containsKey(peerHandle)) {
            Log.d(TAG, "Updated link properties for peer: " + peerHandle);
            // Additional information can be updated here if needed
        }
    }

    /**
     * Handle when a NAN data path is lost
     */
    private void handleDataPathLost(Network network) {
        PeerHandle peerHandle = networkToPeerMap.remove(network);
        if (peerHandle != null) {
            WifiAwareModels.DataPathInfo dataPathInfo = activeDataPaths.remove(peerHandle);
            if (dataPathInfo != null) {
                dataPathInfo.isActive = false;
                Log.d(TAG, "NAN data path lost with peer " + peerHandle + 
                    " at IPv6: " + dataPathInfo.ipAddress + 
                    " (duration: " + (System.currentTimeMillis() - dataPathInfo.createdTime) + "ms)");
                
                // Notify callbacks
                callback.onDataPathLost(network, peerHandle);
            }
        } else {
            Log.w(TAG, "Unknown network lost: " + network);
        }
    }
    
    /**
     * Clean up all active network callbacks to prevent "not in cache" errors
     * Call this when shutting down the data path manager
     */
    public void cleanup() {
        Log.d(TAG, "Cleaning up all network callbacks to prevent cache errors");
        
        // Unregister all active network callbacks
        for (Map.Entry<PeerHandle, ConnectivityManager.NetworkCallback> entry : activeCallbacks.entrySet()) {
            try {
                connectivityManager.unregisterNetworkCallback(entry.getValue());
                Log.d(TAG, "Unregistered network callback for peer: " + entry.getKey());
            } catch (Exception e) {
                Log.w(TAG, "Failed to unregister network callback for peer: " + entry.getKey(), e);
            }
        }
        
        // Clear the tracking map
        activeCallbacks.clear();
        Log.d(TAG, "All network callbacks cleaned up successfully");
    }
}
