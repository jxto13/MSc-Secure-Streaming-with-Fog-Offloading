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
import android.net.NetworkSpecifier;
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

import java.io.FileDescriptor;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.net.Inet6Address;
import java.net.ServerSocket;
import java.net.SocketTimeoutException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.Arrays;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;
import java.net.Socket;
import java.net.InetAddress;
import java.net.Inet6Address;
import java.net.ServerSocket;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.IOException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

import io.grpc.okhttp.internal.Util;
import pt.iris.network.lib.WifiAwareModels.DeviceRole;
import pt.iris.network.lib.WifiAwareModels.DataPathInfo;
import pt.iris.network.lib.WifiAwareModels.SessionInfo;
import pt.iris.network.lib.WifiAwareModels.SessionState;
import pt.iris.network.utils.Utils;
import pt.iris.services.RustService;
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

    private static final int PING_INTERVAL_MS = 5000; // 5 seconds
    private final ScheduledExecutorService pingExecutor = Executors.newScheduledThreadPool(1);
    private final Map<PeerHandle, ServerSocket> serverSockets = new ConcurrentHashMap<>();

    // Track sockets for each peer
    private final Map<PeerHandle, Socket> peerSockets = new ConcurrentHashMap<>();

    private WifiAwareCallback wifiAwareCallback = null;

    // Use per-peer NetworkCallback to avoid duplicate/registered errors
    private final ConcurrentHashMap<PeerHandle, ConnectivityManager.NetworkCallback> networkCallbacks = new ConcurrentHashMap<>();

    // State
    private final UUID deviceUUID = AppContainer.getInstance().getMyUUID();

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


        PublishConfig config = new PublishConfig.Builder()
                .setServiceName(SERVICE_NAME)
                .setServiceSpecificInfo(asBytes(deviceUUID))
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

                notifyDiscoveredPeersChanged();

                switch (new String(message)) {
                    case "CONNECT":
                        establishDataPath(peerHandle, true);
                        publishSession.sendMessage(peerHandle, 0, "NETWORK".getBytes());
                        break;
                }

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
   
                if (!Utils.isMyUuidLower(deviceUUID, Utils.asUuid(serviceSpecificInfo))) {

                    new Handler(Looper.getMainLooper()).postDelayed(() -> {
                        subscribeSession.sendMessage(peerHandle,0, "CONNECT".getBytes());
                    }, 700);
                }

            }

            @Override
            public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
                Log.d(TAG, "Message received from: " + peerHandle);
                switch (new String(message)) {
                    case "NETWORK":
                        establishDataPath(peerHandle, false);
                        break;
                }
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

    private void handlePeerLost(PeerHandle peerHandle) {
        Log.d(TAG, "Handling peer lost: " + peerHandle);

        // Remove peer from discovered peers and active connections
        DataPathInfo removedConnection = activeConnections.remove(peerHandle);

        if (removedConnection != null) {
            wifiAwareCallback.onConnectionLost(peerHandle);
        }

        // Update UI (discovered peers update removed)
        notifyActiveConnectionsChanged();
    }


    public void establishDataPath(PeerHandle peerHandle, boolean masterIsInitiator) {
        Log.d(TAG, "[establishDataPath] CALLED for peer=" + peerHandle + ", masterIsInitiator=" + masterIsInitiator);
        try {
            DiscoverySession currentSession = masterIsInitiator ? publishSession : subscribeSession;

            if (masterIsInitiator) {
                // PUBLISHER/SERVER SIDE
                try {
                    ServerSocket serverSocket = new ServerSocket(0);
                    serverSocket.setReuseAddress(true);
                    serverSockets.put(peerHandle, serverSocket);

                    int port = serverSocket.getLocalPort();
                    Log.d(TAG, "[establishDataPath] ServerSocket created on port: " + port);

                    NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(currentSession, peerHandle)
                            .setPskPassphrase("IrisWifiAware2024")
                            .setPort(port)
                            .build();

                    NetworkRequest request = new NetworkRequest.Builder()
                            .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
                            .setNetworkSpecifier(networkSpecifier)
                            .build();

                    ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
                        @Override
                        public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
                            WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();
                            if (peerAwareInfo != null) {
                                Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
                                Log.d(TAG, "Peer connection info - IP: " + peerIpv6);

                                RustService.addSocket(peerIpv6.getHostAddress());

                                DataPathInfo dataPath = new DataPathInfo(peerHandle, network, peerIpv6.getHostAddress());
                                activeConnections.put(peerHandle, dataPath);

                                Log.d(TAG, "Data path established successfully with peer: " + peerHandle);

                                // Send port to the client before accepting
                                if (publishSession != null) {
                                    String portMsg = "PORT:" + port;
                                    publishSession.sendMessage(peerHandle, 0, portMsg.getBytes());
                                    Log.d(TAG, "Sent port to client: " + portMsg);
                                }

                                // Accept connection in a background thread
                                new Thread(() -> acceptClientSocket(peerHandle, serverSocket)).start();

                                wifiAwareCallback.onConnectionEstablished(peerHandle, dataPath);
                            }
                        }

                        @Override
                        public void onLost(Network network) {
                            cleanupConnection(peerHandle);
                        }

                        @Override
                        public void onUnavailable() {
                            notifyError("Wi-Fi Aware network unavailable for peer: " + peerHandle);
                            unregisterPeerNetworkCallback(peerHandle);
                        }
                    };

                    networkCallbacks.put(peerHandle, callback);
                    connectivityManager.requestNetwork(request, callback);

                } catch (IOException e) {
                    Log.e(TAG, "Failed to create ServerSocket for peer: " + peerHandle, e);
                }
            } else {
                // SUBSCRIBER/CLIENT SIDE
                NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(currentSession, peerHandle)
                        .setPskPassphrase("IrisWifiAware2024")
                        .build();

                NetworkRequest request = new NetworkRequest.Builder()
                        .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
                        .setNetworkSpecifier(networkSpecifier)
                        .build();

                ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
                    @Override
                    public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
                        WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();
                        if (peerAwareInfo != null) {
                            Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
                            int peerPort = peerAwareInfo.getPort();
                            Log.d(TAG, "Connecting to server socket at: " + peerIpv6 + ":" + peerPort);

                            RustService.addSocket(peerIpv6.getHostAddress());

                            DataPathInfo dataPath = new DataPathInfo(peerHandle, network, peerIpv6.getHostAddress());
                            activeConnections.put(peerHandle, dataPath);

                            try {
                                Socket socket = network.getSocketFactory().createSocket(peerIpv6, peerPort);
                                socket.setSoTimeout(15000); // Timeout for read operations
                                peerSockets.put(peerHandle, socket);
                                Log.d(TAG, "Connected to server socket for peer: " + peerHandle);

                                // Start pinging periodically
                                startPinging(peerHandle, socket);

                            } catch (IOException e) {
                                Log.e(TAG, "Failed to connect to server socket for peer: " + peerHandle, e);
                            }

                            wifiAwareCallback.onConnectionEstablished(peerHandle, dataPath);
                        }
                    }

                    @Override
                    public void onLost(Network network) {
                        cleanupConnection(peerHandle);
                    }

                    @Override
                    public void onUnavailable() {
                        notifyError("Wi-Fi Aware network unavailable for peer: " + peerHandle);
                        unregisterPeerNetworkCallback(peerHandle);
                    }
                };

                networkCallbacks.put(peerHandle, callback);
                connectivityManager.requestNetwork(request, callback);
            }
        } catch (Exception e) {
            Log.e(TAG, "Unexpected error during data path establishment: " + e.getMessage(), e);
            notifyError("Unexpected error: " + e.getMessage());
        }
    }

    private void acceptClientSocket(PeerHandle peerHandle, ServerSocket serverSocket) {
        try {
            serverSocket.setSoTimeout(10000); // Wait up to 10 seconds per accept attempt

            boolean connected = false;
            long startTime = System.currentTimeMillis();

            while (!connected && System.currentTimeMillis() - startTime < 30000) { // Retry for 30 seconds max
                try {
                    Socket clientSocket = serverSocket.accept(); // Blocking wait
                    clientSocket.setSoTimeout(15000);
                    peerSockets.put(peerHandle, clientSocket);
                    Log.d(TAG, "Accepted client socket for peer: " + peerHandle);

                    // Start pinging from server if desired
                    startPinging(peerHandle, clientSocket);
                    connected = true;
                } catch (SocketTimeoutException e) {
                    Log.w(TAG, "Accept timeout for peer: " + peerHandle + " - retrying...");
                }
            }

            if (!connected) {
                Log.e(TAG, "Failed to accept client socket for peer: " + peerHandle + " after retries");
            }

        } catch (IOException e) {
            Log.e(TAG, "Error accepting client socket for peer: " + peerHandle, e);
        }
    }


    private void startPinging(PeerHandle peerHandle, Socket socket) {
        pingExecutor.scheduleWithFixedDelay(() -> {
            try {
                if (socket != null && socket.isConnected() && !socket.isClosed()) {
                    OutputStream out = socket.getOutputStream();
                    out.write("PING\n".getBytes());
                    out.flush();
                    Log.d(TAG, "Ping sent to peer: " + peerHandle);
                }
            } catch (IOException e) {
                Log.e(TAG, "Ping failed for peer: " + peerHandle, e);
                cleanupConnection(peerHandle);
            }
        }, PING_INTERVAL_MS, PING_INTERVAL_MS, TimeUnit.MILLISECONDS);
    }

    private void cleanupConnection(PeerHandle peerHandle) {
        Log.d(TAG, "Cleaning up connection for peer: " + peerHandle);
        DataPathInfo removedPath = activeConnections.remove(peerHandle);
        if (removedPath != null) removedPath.isActive = false;
        wifiAwareCallback.onConnectionLost(peerHandle);

        Socket s = peerSockets.remove(peerHandle);
        if (s != null) try { s.close(); } catch (IOException ignore) {}

        ServerSocket server = serverSockets.remove(peerHandle);
        if (server != null) try { server.close(); } catch (IOException ignore) {}

        unregisterPeerNetworkCallback(peerHandle);
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


    public Map<PeerHandle, DataPathInfo> getAllPeers() {
		Map<PeerHandle, DataPathInfo> allPeers = new ConcurrentHashMap<>(activeConnections);
        return allPeers;
    }



    public SessionInfo getSessionInfo() {
        SessionState sessionState = SessionState.IDLE;

        return new SessionInfo(
                sessionState,
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
        // Removed discovered peers UI update
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

        for (PeerHandle peerHandle : activeConnections.keySet()) {
            wifiAwareCallback.onConnectionLost(peerHandle);
        }

        // Clear peer lists
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

        // Clean up all sockets
        for (Socket s : peerSockets.values()) {
            try { s.close(); } catch (IOException ignore) {}
        }
        peerSockets.clear();

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