

package pt.iris.network.lib;

import static pt.iris.network.utils.Utils.asBytes;

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
import android.net.wifi.aware.Characteristics;
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
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import java.io.IOException;
import java.net.Inet6Address;
import java.net.ServerSocket;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.Arrays;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;

import pt.iris.network.lib.WifiAwareModels.DeviceRole;
import pt.iris.network.lib.WifiAwareModels.DataPathInfo;
import pt.iris.network.lib.WifiAwareModels.SessionInfo;
import pt.iris.network.lib.WifiAwareModels.SessionState;
import pt.iris.utils.AppContainer;

/**
 * Complete Wi-Fi Aware implementation for Android
 * Handles discovery, publishing, messaging, and data path connections
 */
@SuppressLint("MissingPermission")
public class WifiAwareLibrary {
    private static final String TAG = "WifiAwareLibrary";
    public static final String SERVICE_NAME = "Iris_Streaming_Service";

    private final Context context;
    public final WifiAwareManager wifiAwareManager;
    private final ConnectivityManager connectivityManager;

    // Session management
    private WifiAwareSession wifiAwareSession;
    private PublishDiscoverySession publishSession;
    private SubscribeDiscoverySession subscribeSession;

    // Connection tracking
    private final Map<PeerHandle, DataPathInfo> activeConnections = new ConcurrentHashMap<>();
    private final Map<PeerHandle, DataPathInfo> discoveredPeers = new ConcurrentHashMap<>();
    private WifiAwareCallback wifiAwareCallback = null;

    // Use per-peer NetworkCallback to avoid duplicate/registered errors
    private final ConcurrentHashMap<PeerHandle, ConnectivityManager.NetworkCallback> networkCallbacks = new ConcurrentHashMap<>();

    // State
    private final UUID deviceUUID = AppContainer.getInstance().getMyUUID();
    private WifiAwareModels.DeviceCapabilities myCapabilities;
    private final Map<PeerHandle, WifiAwareModels.DeviceCapabilities> peerCapabilities = new ConcurrentHashMap<>();
    // Track role and UUID per peer
    private final Map<PeerHandle, DeviceRole> peerRoles = new ConcurrentHashMap<>();
    private final Map<PeerHandle, UUID> peerUuids = new ConcurrentHashMap<>();
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Map<PeerHandle, Integer> connectionRetryCounts = new ConcurrentHashMap<>();
    private static final int MAX_RETRIES = 3;
    private static final int RETRY_DELAY_MS = 3000;

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

                wifiAwareCallback.onWifiAwareAvailable(isAvailable);


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
        wifiAwareCallback = callback;
    }

    public void removeCallback() {
        wifiAwareCallback = null;
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
                wifiAwareSession = session;

                wifiAwareCallback.onSessionAttached(session);
            }

            @Override
            public void onAttachFailed() {
                Log.e(TAG, "Failed to attach to Wi-Fi Aware");
                notifyError("Failed to attach to Wi-Fi Aware");
            }
        }, null);
    }

    public void startPublishing() {
        if (wifiAwareSession == null) {
            notifyError("Wi-Fi Aware session not available");
            return;
        }

        // Only advertise static identity (UUID) in serviceSpecificInfo
        byte[] uuidBytes = asBytes(deviceUUID);
        PublishConfig config = new PublishConfig.Builder()
                .setServiceName(SERVICE_NAME)
                .setServiceSpecificInfo(uuidBytes)
                .build();

        wifiAwareSession.publish(config, new DiscoverySessionCallback() {
            @Override
            public void onPublishStarted(PublishDiscoverySession session) {
                Log.d(TAG, "Publishing started");
                publishSession = session;

                wifiAwareCallback.onPublishStarted(session);
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
                handleSessionTermination();
            }
        }, null);
    }

    public void startSubscribing() {
        if (wifiAwareSession == null) {
            notifyError("Wi-Fi Aware session not available");
            return;
        }

        byte[] uuidBytes = asBytes(deviceUUID);
        SubscribeConfig config = new SubscribeConfig.Builder()
                .setServiceName(SERVICE_NAME)
                .setServiceSpecificInfo(uuidBytes)
                .build();
        wifiAwareSession.subscribe(config, new DiscoverySessionCallback() {

            @Override
            public void onSubscribeStarted(SubscribeDiscoverySession session) {
                Log.d(TAG, "Subscribing started");
                subscribeSession = session;

                wifiAwareCallback.onSubscribeStarted(session);
            }

            @Override
            public void onSessionConfigFailed() {
                Log.e(TAG, "Failed to start subscribing");
                notifyError("Failed to start subscribing");
            }

            @Override
            public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
                Log.d(TAG, "Service discovered from peer: " + peerHandle);

                DataPathInfo pathInfo = new DataPathInfo(peerHandle);
                discoveredPeers.put(peerHandle, pathInfo);
                notifyDiscoveredPeersChanged();

                // Notify callbacks about peer discovery
                wifiAwareCallback.onPeerDiscovered(peerHandle);
                // No automatic role assignment or connection. Let user trigger connect.

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
                handleSessionTermination();
            }
        }, null);
    }

    // Helper to send a message to a peer


    private void handleMessageReceived(PeerHandle peerHandle, byte[] message) {
        // No automatic connection on message received. Let user trigger connect.
        Log.d(TAG, "Received message from " + peerHandle + ". No auto-connect. Waiting for user action.");
    }


    // Determine master/slave role based on device capabilities
    private DeviceRole determineRole(UUID myUuid, UUID peerUuid) {
        if (myUuid != null && peerUuid != null) {
            int cmp = myUuid.compareTo(peerUuid);
            Log.d(TAG, "Comparing UUIDs for role: myUuid=" + myUuid + ", peerUuid=" + peerUuid + ", cmp=" + cmp);
            return cmp <= 0 ? DeviceRole.MASTER : DeviceRole.SLAVE;
        } else {
            Log.w(TAG, "[AWARE] One or both UUIDs are null in determineRole. mine=" + myUuid + ", peer=" + peerUuid);
            // Fallback: if peer UUID is missing, prefer SLAVE to avoid collision
            return DeviceRole.SLAVE;
        }
    }

    private void handlePeerLost(PeerHandle peerHandle) {
        Log.d(TAG, "Handling peer lost: " + peerHandle);

        // Remove peer from discovered peers and active connections
        DataPathInfo removedPeer = discoveredPeers.remove(peerHandle);
        DataPathInfo removedConnection = activeConnections.remove(peerHandle);

        if (removedPeer != null) {
            wifiAwareCallback.onPeerLost(peerHandle);
        }
        if (removedConnection != null) {
            wifiAwareCallback.onConnectionLost(peerHandle);
        }

        // Update UI
        notifyDiscoveredPeersChanged();
        notifyActiveConnectionsChanged();

    }

    public void establishDataPath(PeerHandle peerHandle, boolean masterIsInitiator) {
        // masterIsInitiator: true if this device is master and should initiate (use publishSession)
        Log.d(TAG, "[establishDataPath] CALLED for peer=" + peerHandle + ", masterIsInitiator=" + masterIsInitiator);
        Log.d(TAG, "[establishDataPath] publishSession=" + publishSession + ", subscribeSession=" + subscribeSession);
        try {
            DiscoverySession currentSession = masterIsInitiator ? publishSession : subscribeSession;

                Log.d(TAG, "[establishDataPath] Master using publishSession for peer: " + peerHandle);
                WifiAwareNetworkSpecifier.Builder specBuilder = new WifiAwareNetworkSpecifier.Builder(
                        publishSession, peerHandle);
                specBuilder.setPskPassphrase("IrisWifiAware2024");
                WifiAwareNetworkSpecifier networkSpec;
                try {
                    networkSpec = specBuilder.build();
                    Log.d(TAG, "[establishDataPath] Network specifier built successfully");
                } catch (Exception e) {
                    String specError = "Failed to build network specifier: " + e.getMessage() + ". Peer=" + peerHandle;
                    Log.e(TAG, specError, e);
                    notifyError(specError);
                    return;
                }
                NetworkRequest request = new NetworkRequest.Builder()
                        .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
                        .setNetworkSpecifier(networkSpec)
                        .build();
                Log.d(TAG, "[establishDataPath] Requesting WiFi Aware network as master. Peer=" + peerHandle);

                // Always create a new callback for this peer
                Log.d(TAG, "[establishDataPath] About to call connectivityManager.requestNetwork for peer=" + peerHandle);
                ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
                    @Override
                    public void onAvailable(Network network) {
                        Log.d(TAG, "Wi-Fi Aware network available for peer: " + peerHandle);
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

                            wifiAwareCallback.onConnectionEstablished(peerHandle, dataPath);
                        }
                    }

                    @Override
                    public void onLost(Network network) {
                        Log.d(TAG, "Wi-Fi Aware network lost for peer: " + peerHandle);
                        DataPathInfo removedPath = activeConnections.remove(peerHandle);
                        if (removedPath != null) {
                            removedPath.isActive = false;
                            wifiAwareCallback.onConnectionLost(peerHandle);
                        }
                        // Unregister and remove callback
                        unregisterPeerNetworkCallback(peerHandle);
                    }

                    @Override
                    public void onUnavailable() {
                        Log.e(TAG, "Wi-Fi Aware network unavailable for peer: " + peerHandle);
                        notifyError("Wi-Fi Aware network unavailable for peer: " + peerHandle);
                        // Unregister and remove callback
                        unregisterPeerNetworkCallback(peerHandle);
                    }
                };
                // Store callback for this peer
                networkCallbacks.put(peerHandle, callback);
                try {
                    connectivityManager.requestNetwork(request, callback);
                    Log.d(TAG, "[establishDataPath] requestNetwork called successfully for peer: " + peerHandle);
                } catch (Exception e) {
                    String errMsg = "Exception during requestNetwork: " + e.getMessage() + ". Peer=" + peerHandle;
                    Log.e(TAG, errMsg, e);
                    notifyError(errMsg);
                    unregisterPeerNetworkCallback(peerHandle);
                }

        } catch (Exception e) {
            String errorLog = "Unexpected error during data path establishment: " + e.getMessage();
            Log.e(TAG, errorLog, e);
            notifyError(errorLog);
        }
    }

    // Helper to unregister and remove callback for a peer
    private void unregisterPeerNetworkCallback(PeerHandle peerHandle) {
        ConnectivityManager.NetworkCallback cb = networkCallbacks.remove(peerHandle);
        if (cb != null && connectivityManager != null) {
            try {
                connectivityManager.unregisterNetworkCallback(cb);
            } catch (Exception e) {
                Log.w(TAG, "Exception while unregistering network callback for peer: " + peerHandle, e);
            }
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



    public SessionInfo getSessionInfo() {
        SessionState sessionState = SessionState.IDLE;

        return new SessionInfo(
                sessionState,
                discoveredPeers.size(),
                activeConnections.size()
        );
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
        wifiAwareCallback.onError(error);

    }


    private void handleSessionTermination() {
        Log.d(TAG, "Handling session termination - cleaning up peers");

        // Notify about lost peers
        for (PeerHandle peerHandle : discoveredPeers.keySet()) {
            wifiAwareCallback.onPeerLost(peerHandle);
        }

        for (PeerHandle peerHandle : activeConnections.keySet()) {
            wifiAwareCallback.onConnectionLost(peerHandle);
        }

        // Clear peer lists
        discoveredPeers.clear();
        activeConnections.clear();
    }

    public void cleanup() {
        Log.d(TAG, "Cleaning up Wi-Fi Aware resources");

        // Close discovery sessions
        if (publishSession != null) {
            publishSession.close();
            publishSession = null;
        }

        if (subscribeSession != null) {
            subscribeSession.close();
            subscribeSession = null;
        }

        // Close Wi-Fi Aware session
        if (wifiAwareSession != null) {
            wifiAwareSession.close();
            wifiAwareSession = null;
        }

        // Clean up all network callbacks
        for (PeerHandle peer : networkCallbacks.keySet()) {
            unregisterPeerNetworkCallback(peer);
        }
        networkCallbacks.clear();



        // Mark all connections as inactive
        for (DataPathInfo path : activeConnections.values()) {
            path.isActive = false;
        }
        activeConnections.clear();
        discoveredPeers.clear();

    }

    public void destroy() {
        cleanup();

        // Unregister broadcast receiver
        try {
            context.unregisterReceiver(wifiAwareStateReceiver);
        } catch (Exception e) {
            Log.w(TAG, "Error unregistering broadcast receiver", e);
        }
    }

}