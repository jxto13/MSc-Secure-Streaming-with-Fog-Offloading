package pt.iris.network.lib;

import android.annotation.SuppressLint;
import android.net.wifi.ScanResult;
import android.net.wifi.aware.Characteristics;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Build;
import android.util.Log;

import java.util.List;
import java.util.Map;
import java.util.UUID;

/**
 * Manages WiFi Aware discovery sessions (publish/subscribe)
 */
@SuppressLint("MissingPermission")
public class WifiAwareDiscoveryManager {
    private static final String TAG = "WifiAwareDiscoveryManager";

    private final WifiAwareManager wifiAwareManager;
    private final String serviceName;
    private final UUID deviceUUID;
    private final DiscoveryCallback callback;
    private final Map<PeerHandle, WifiAwareModels.DeviceCapabilities> peerCapabilities;

    private PublishDiscoverySession publisherSession;
    private SubscribeDiscoverySession subscriberSession;

    public interface DiscoveryCallback {
        WifiAwareModels.DeviceCapabilities getMyCapabilities();
        void onCapabilityExchange(PeerHandle peerHandle, WifiAwareModels.DeviceCapabilities peerCapabilities, boolean asPublisher);
        void onServiceLost(PeerHandle peerHandle);
    }

    public WifiAwareDiscoveryManager(WifiAwareManager wifiAwareManager,
                                   String serviceName,
                                   UUID deviceUUID,
                                   DiscoveryCallback callback,
                                   Map<PeerHandle, WifiAwareModels.DeviceCapabilities> peerCapabilities) {
        this.wifiAwareManager = wifiAwareManager;
        this.serviceName = serviceName;
        this.deviceUUID = deviceUUID;
        this.callback = callback;
        this.peerCapabilities = peerCapabilities;
    }

    /**
     * Start publisher with proper data path management
     */
    @SuppressLint("WrongConstant")
    public void startPublisher(WifiAwareSession session) {
        if (publisherSession != null) {
            Log.w(TAG, "Publisher already active");
            return;
        }

        PublishConfig.Builder configBuilder = new PublishConfig.Builder()
                .setServiceName(serviceName)
                .setServiceSpecificInfo(callback.getMyCapabilities().toBytes())
                .setTerminateNotificationEnabled(true);

        // Enable instant communication mode if supported (Android 13+)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            Characteristics characteristics = wifiAwareManager.getCharacteristics();
            if (characteristics.isInstantCommunicationModeSupported()) {
                Log.d(TAG, "Enabling instant communication mode for publisher");
                configBuilder.setInstantCommunicationModeEnabled(true, ScanResult.WIFI_BAND_24_GHZ);
            }
        }

        PublishConfig config = configBuilder.build();

        session.publish(config, new DiscoverySessionCallback() {
            @Override
            public void onPublishStarted(PublishDiscoverySession session) {
                Log.d(TAG, "Publisher started successfully with session ID: " + session.hashCode());
                publisherSession = session;
            }

            @Override
            public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
                Log.d(TAG, "Publisher received message from peer: " + peerHandle);

                // Parse capabilities from message
                WifiAwareModels.DeviceCapabilities peerCapabilities = WifiAwareModels.DeviceCapabilities.fromBytes(message);
                if (peerCapabilities != null) {
                    WifiAwareDiscoveryManager.this.peerCapabilities.put(peerHandle, peerCapabilities);
                    callback.onCapabilityExchange(peerHandle, peerCapabilities, true);
                } else {
                    Log.w(TAG, "Invalid capabilities received from peer: " + peerHandle);
                }
            }

            @Override
            public void onSessionTerminated() {
                Log.w(TAG, "Publisher session terminated");
                publisherSession = null;
            }
        }, null);
    }

    /**
     * Start subscriber with proper data path management
     */
    public void startSubscriber(WifiAwareSession session) {
        if (subscriberSession != null) {
            Log.w(TAG, "Subscriber already active");
            return;
        }

        SubscribeConfig.Builder configBuilder = new SubscribeConfig.Builder()
                .setServiceName(serviceName)
                .setTerminateNotificationEnabled(true);

        // Enable instant communication mode if supported (Android 13+)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            Characteristics characteristics = wifiAwareManager.getCharacteristics();
            if (characteristics.isInstantCommunicationModeSupported()) {
                Log.d(TAG, "Enabling instant communication mode for subscriber");
                configBuilder.setInstantCommunicationModeEnabled(true, ScanResult.WIFI_BAND_24_GHZ);
            }
        }

        SubscribeConfig config = configBuilder.build();

        session.subscribe(config, new DiscoverySessionCallback() {
            @Override
            public void onSubscribeStarted(SubscribeDiscoverySession session) {
                Log.d(TAG, "Subscriber started successfully with session ID: " + session.hashCode());
                subscriberSession = session;
            }

            @Override
            public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
                Log.d(TAG, "Service discovered from peer: " + peerHandle);

                // Parse capabilities from service specific info
                WifiAwareModels.DeviceCapabilities peerCapabilities = WifiAwareModels.DeviceCapabilities.fromBytes(serviceSpecificInfo);
                if (peerCapabilities != null) {
                    Log.d(TAG, "Peer capabilities - Interfaces: " + peerCapabilities.availableInterfaces +
                            ", Data Paths: " + peerCapabilities.availableDataPaths +
                            ", UUID: " + peerCapabilities.deviceUUID);

                    // Store peer capabilities
                    WifiAwareDiscoveryManager.this.peerCapabilities.put(peerHandle, peerCapabilities);

                    // Send our capabilities back
                    byte[] myCapabilitiesBytes = callback.getMyCapabilities().toBytes();
                    subscriberSession.sendMessage(peerHandle, 0, myCapabilitiesBytes);

                    // Start connection process immediately after capability exchange
                    callback.onCapabilityExchange(peerHandle, peerCapabilities, false);
                } else {
                    Log.w(TAG, "Invalid capabilities in service discovery from peer: " + peerHandle);
                    // Fallback to sending empty message for legacy compatibility
                    subscriberSession.sendMessage(peerHandle, 0, new byte[0]);
                }
            }

            @Override
            public void onServiceLost(PeerHandle peerHandle, int reason) {
                Log.d(TAG, "Service lost from peer: " + peerHandle + ", reason: " + reason);
                // Clean up peer capabilities
                WifiAwareDiscoveryManager.this.peerCapabilities.remove(peerHandle);
                callback.onServiceLost(peerHandle);
            }

            @Override
            public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
                Log.d(TAG, "Subscriber received message from peer: " + peerHandle);

                // Parse capabilities from message
                WifiAwareModels.DeviceCapabilities peerCapabilities = WifiAwareModels.DeviceCapabilities.fromBytes(message);
                if (peerCapabilities != null) {
                    WifiAwareDiscoveryManager.this.peerCapabilities.put(peerHandle, peerCapabilities);
                    callback.onCapabilityExchange(peerHandle, peerCapabilities, false);
                } else {
                    Log.w(TAG, "Invalid capabilities received from peer: " + peerHandle);
                }
            }

            @Override
            public void onSessionTerminated() {
                Log.w(TAG, "Subscriber session terminated");
                subscriberSession = null;
            }
        }, null);
    }

    /**
     * Stop publisher and clean up
     */
    public void stopPublisher() {
        if (publisherSession != null) {
            publisherSession.close();
            publisherSession = null;
            Log.d(TAG, "Publisher stopped");
        }
    }

    /**
     * Stop subscriber and clean up
     */
    public void stopSubscriber() {
        if (subscriberSession != null) {
            subscriberSession.close();
            subscriberSession = null;
            Log.d(TAG, "Subscriber stopped");
        }
    }

    /**
     * Get publisher session
     */
    public PublishDiscoverySession getPublisherSession() {
        return publisherSession;
    }

    /**
     * Get subscriber session
     */
    public SubscribeDiscoverySession getSubscriberSession() {
        return subscriberSession;
    }

    /**
     * Check if publisher is active
     */
    public boolean isPublisherActive() {
        return publisherSession != null;
    }

    /**
     * Check if subscriber is active
     */
    public boolean isSubscriberActive() {
        return subscriberSession != null;
    }
}
