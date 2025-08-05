
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

import androidx.annotation.NonNull;

import java.io.FileDescriptor;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.net.Inet6Address;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.SocketTimeoutException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.Arrays;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.CopyOnWriteArraySet;
import java.net.InetAddress;
import java.net.InetSocketAddress;
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

    private static final int TCP_PORT = 8080;
    private final Map<PeerHandle, ServerSocket> serverSockets = new ConcurrentHashMap<>();
    private final Map<PeerHandle, Socket> clientSockets = new ConcurrentHashMap<>();

    // Keepalive ping mechanism
    private static final int PING_INTERVAL_MS = 30000; // 30 seconds
    private final ScheduledExecutorService pingExecutor = Executors.newScheduledThreadPool(2);
    private final Set<PeerHandle> activePings = new CopyOnWriteArraySet<>();
    
    // Track which peers we're actively connected to
    private final Set<PeerHandle> activeConnections2 = new CopyOnWriteArraySet<>();

    private WifiAwareCallback wifiAwareCallback = null;

    // Socket callback interface for immediate socket notifications
    public interface SocketCallback {
        void onNewSocketAvailable(Object peerId, Socket socket);
        void onSocketClosed(Object peerId);
    }
    
    private SocketCallback socketCallback = null;

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
                        public void onAvailable(@NonNull Network network){
//                            connectivityManager.bindProcessToNetwork(network);
                        }

                        @Override
                        public void onCapabilitiesChanged(@NonNull Network network, NetworkCapabilities networkCapabilities) {
                            WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();
                            if (peerAwareInfo != null) {
                                Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();

                                DataPathInfo dataPath = new DataPathInfo(peerHandle, network, peerIpv6.getHostAddress());
                                activeConnections.put(peerHandle, dataPath);

                                RustService.addSocket(peerIpv6.getHostAddress());


                                // Send port to the client before starting to listen
                                if (publishSession != null) {
                                    String portMsg = "PORT:" + port;
                                    publishSession.sendMessage(peerHandle, 0, portMsg.getBytes());
                                    Log.d(TAG, "Sent port to client: " + portMsg);
                                }

                                // Start listening for client connections
                                new Thread(() -> {
                                    try {
                                        // Set a reasonable timeout for accept operations
                                        serverSocket.setSoTimeout(30000); // 30 seconds
                                        Log.d(TAG, "Server waiting for client connection on port: " + port);
                                        
                                        Socket clientSocket = serverSocket.accept();
                                        clientSockets.put(peerHandle, clientSocket);
                                        Log.d(TAG, "Client connected from: " + clientSocket.getRemoteSocketAddress());
                                        
                                        // Notify VPN service about new socket
                                        if (socketCallback != null) {
                                            socketCallback.onNewSocketAvailable(peerHandle, clientSocket);
                                        }
                                        
                                        // Start continuous packet listener and keepalive pings
                                        startContinuousPacketListener(peerHandle, clientSocket);
                                        startKeepAlivePing(peerHandle, clientSocket);
                                        
                                    } catch (java.net.SocketTimeoutException e) {
                                        Log.w(TAG, "Timeout waiting for client connection from peer: " + peerHandle);
                                    } catch (IOException e) {
                                        Log.e(TAG, "Error accepting client connection", e);
                                    }
                                }, "ServerAccept-" + peerHandle).start();

                                Log.d(TAG, "Server side: ServerSocket created for peer " + peerHandle + " on port " + port);

                                wifiAwareCallback.onConnectionEstablished(peerHandle, dataPath);
                            }
                        }

                        @Override
                        public void onLost(@NonNull Network network) {
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
                    public void onAvailable(@NonNull Network network){
//                        connectivityManager.bindProcessToNetwork(network);
                    }

                    @Override
                    public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
                        WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();
                        if (peerAwareInfo != null) {
                            Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
                            int peerPort = peerAwareInfo.getPort();

                            DataPathInfo dataPath = new DataPathInfo(peerHandle, network, peerIpv6.getHostAddress());
                            activeConnections.put(peerHandle, dataPath);

                            RustService.addSocket(peerIpv6.getHostAddress());

                            // Connect to server using network socket factory
                            new Thread(() -> {
                                try {
                                    Log.d(TAG, "Connecting to server at: " + peerIpv6 + ":" + peerPort);
                                    Socket socket = network.getSocketFactory().createSocket(peerIpv6, peerPort);
                                    
                                    clientSockets.put(peerHandle, socket);
                                    Log.d(TAG, "Client connected to server at: " + socket.getRemoteSocketAddress());
                                    
                                    // Notify VPN service about new socket
                                    if (socketCallback != null) {
                                        socketCallback.onNewSocketAvailable(peerHandle, socket);
                                    }
                                    
                                    // Start continuous packet listener and keepalive pings
                                    startContinuousPacketListener(peerHandle, socket);
                                    startKeepAlivePing(peerHandle, socket);
                                    
                                } catch (IOException e) {
//                                    Log.e(TAG, "Failed to connect to server at " + peerIpv6 + ":" + peerPort, e);
                                }
                            }, "ClientConnect-" + peerHandle).start();

                            Log.d(TAG, "Client side: Socket connection initiated for peer " + peerHandle);

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

    /**
     * Starts continuous packet listener for the TCP socket
     */
    private void startContinuousPacketListener(PeerHandle peerHandle, Socket socket) {
        new Thread(() -> {
            try {
                // Socket is now established and ready for data exchange
                Log.d(TAG, "TCP connection established with peer: " + peerHandle);
                activeConnections2.add(peerHandle);
                
                // Keep connection alive but don't read data here
                // Data reading will be handled by VPN service to avoid conflicts
                while (!socket.isClosed() && socket.isConnected()) {
                    try {
                        Thread.sleep(5000); // Check connection every 5 seconds
                        if (!socket.isConnected()) {
                            break;
                        }
                    } catch (InterruptedException e) {
                        Log.d(TAG, "TCP listener interrupted for peer: " + peerHandle);
                        break;
                    }
                }
                
            } catch (Exception e) {
                Log.w(TAG, "TCP connection monitor error for peer " + peerHandle, e);
            }
            finally {
                activeConnections2.remove(peerHandle);
            }
            Log.d(TAG, "TCP connection monitor exiting for peer: " + peerHandle);
        }, "TCPMonitor-" + peerHandle).start();
    }

    /**
     * Starts keepalive ping mechanism for TCP socket
     */
    private void startKeepAlivePing(PeerHandle peerHandle, Socket socket) {
        if (activePings.contains(peerHandle)) {
            Log.d(TAG, "Already pinging peer: " + peerHandle);
            return;
        }
        
        activePings.add(peerHandle);
        Log.d(TAG, "Starting keepalive ping for peer: " + peerHandle);
        
        pingExecutor.scheduleWithFixedDelay(() -> {
            try {
                if (socket != null && !socket.isClosed() && socket.isConnected()) {
                    // Send a simple ping message
                    String pingMessage = "PING_KEEPALIVE\n";
                    byte[] pingData = pingMessage.getBytes();
                    
                    socket.getOutputStream().write(pingData);
                    socket.getOutputStream().flush();
                    
                    Log.v(TAG, "Keepalive ping sent to peer: " + peerHandle);
                } else {
                    Log.w(TAG, "Socket closed for peer: " + peerHandle + ", stopping pings");
                    activePings.remove(peerHandle);
                }
            } catch (IOException e) {
                Log.e(TAG, "Keepalive ping failed for peer: " + peerHandle, e);
                activePings.remove(peerHandle);
                cleanupConnection(peerHandle);
            }
        }, PING_INTERVAL_MS, PING_INTERVAL_MS, TimeUnit.MILLISECONDS);
    }

    /**
     * Gets the local TUN IP address
     */
    private String getLocalTunIP() {
        return localTunIP;
    }

    private void cleanupConnection(PeerHandle peerHandle) {
        Log.d(TAG, "Cleaning up connection for peer: " + peerHandle);
        DataPathInfo removedPath = activeConnections.remove(peerHandle);
        if (removedPath != null) removedPath.isActive = false;
        wifiAwareCallback.onConnectionLost(peerHandle);

        // Close ServerSocket if exists
        ServerSocket serverSocket = serverSockets.remove(peerHandle);
        if (serverSocket != null && !serverSocket.isClosed()) {
            try {
                serverSocket.close();
            } catch (IOException e) {
                Log.w(TAG, "Error closing ServerSocket for peer: " + peerHandle, e);
            }
        }

        // Close client Socket if exists
        Socket clientSocket = clientSockets.remove(peerHandle);
        if (clientSocket != null && !clientSocket.isClosed()) {
            try {
                clientSocket.close();
            } catch (IOException e) {
                Log.w(TAG, "Error closing client Socket for peer: " + peerHandle, e);
            }
        }
        
        // Notify VPN service about socket closure
        if (socketCallback != null) {
            socketCallback.onSocketClosed(peerHandle);
        }
        
        // Clean up peer data
        activeConnections2.remove(peerHandle);
        activePings.remove(peerHandle);

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


    /**
     * Returns a copy of the current peer ServerSockets map.
     */
    public Map<PeerHandle, ServerSocket> getPeerServerSockets() {
        return new ConcurrentHashMap<>(serverSockets);
    }

    /**
     * Returns a copy of the current peer client Sockets map.
     */
    public Map<PeerHandle, Socket> getPeerClientSockets() {
        return new ConcurrentHashMap<>(clientSockets);
    }

    /**
     * Sends data to a specific peer via TCP (simplified for revert).
     */
    public boolean sendDataToPeer(PeerHandle peerHandle, byte[] data) {
        Socket socket = clientSockets.get(peerHandle);
        
        if (socket == null || socket.isClosed() || !socket.isConnected()) {
            Log.w(TAG, "Cannot send data to peer " + peerHandle + ": socket unavailable");
            return false;
        }
        
        try {
            socket.getOutputStream().write(data);
            socket.getOutputStream().flush();
            Log.d(TAG, "Sent " + data.length + " bytes to peer: " + peerHandle);
            return true;
        } catch (IOException e) {
            Log.e(TAG, "Failed to send data to peer " + peerHandle, e);
            return false;
        }
    }


    /**
     * Sets the local TUN IP address (should be called from VPN service)
     */
    public void setLocalTunIP(String tunIP) {
        // This will be used in getLocalTunIP()
        this.localTunIP = tunIP;
    }

    /**
     * Sets the socket callback for immediate notification of new sockets
     */
    public void setSocketCallback(SocketCallback callback) {
        this.socketCallback = callback;
        Log.d(TAG, "Socket callback set: " + (callback != null ? "enabled" : "disabled"));
    }

    private String localTunIP = null;

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

        // Clean up all ServerSockets
        for (ServerSocket serverSocket : serverSockets.values()) {
            if (!serverSocket.isClosed()) {
                try {
                    serverSocket.close();
                } catch (IOException e) {
                    Log.w(TAG, "Error closing ServerSocket during cleanup", e);
                }
            }
        }
        serverSockets.clear();

        // Clean up all client Sockets
        for (Socket clientSocket : clientSockets.values()) {
            if (!clientSocket.isClosed()) {
                try {
                    clientSocket.close();
                } catch (IOException e) {
                    Log.w(TAG, "Error closing client Socket during cleanup", e);
                }
            }
        }
        clientSockets.clear();

        // Clean up peer data
        activeConnections2.clear();
        activePings.clear();

        // Shutdown ping executor
        if (pingExecutor != null && !pingExecutor.isShutdown()) {
            pingExecutor.shutdown();
            try {
                if (!pingExecutor.awaitTermination(5, TimeUnit.SECONDS)) {
                    pingExecutor.shutdownNow();
                }
            } catch (InterruptedException e) {
                pingExecutor.shutdownNow();
                Thread.currentThread().interrupt();
            }
        }

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