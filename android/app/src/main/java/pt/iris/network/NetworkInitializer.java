package pt.iris.network;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareSession;
import android.util.Log;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CopyOnWriteArrayList;

import pt.iris.network.lib.WifiAwareLibrary;
import pt.iris.network.lib.WifiAwareModels;
import pt.iris.network.lib.WifiAwareModels.DataPathInfo;
import pt.iris.network.lib.WifiAwareModels.DeviceRole;

@SuppressLint("MissingPermission")
public class NetworkInitializer implements WifiAwareLibrary.WifiAwareCallback {
	private static final String TAG = "NetworkInitializer";
	public static final String SERVICE_NAME = "Iris_Streaming_Service";

	private final Context context;
	private final WifiAwareLibrary wifiAwareLibrary;
	private final CopyOnWriteArrayList<NetworkConnectionListener> listeners = new CopyOnWriteArrayList<>();
	private UUID myUUID;
	
	// State tracking
	private boolean isInitialized = false;
	private boolean isPublishing = false;
	private boolean isSubscribing = false;

	public interface NetworkConnectionListener {
		void onConnectionEstablished(String ipAddress, PeerHandle peerHandle);
		void onConnectionLost(String ipAddress, PeerHandle peerHandle);
		void onWifiAwareStateChanged(boolean available);
		void onConnectionAttempt(String logMessage);
		void onError(String error);
		void onPeerDiscovered(PeerHandle peerHandle, DataPathInfo peerInfo);
		void onPeerLost(PeerHandle peerHandle);
		void onRoleChanged(DeviceRole newRole);
		void onSessionInfoChanged(WifiAwareModels.SessionInfo sessionInfo);
	}

	public NetworkInitializer(Context context) {
		this.context = context;
		this.wifiAwareLibrary = new WifiAwareLibrary(context);
		this.myUUID = UUID.randomUUID();
		
		// Register this class as a callback for Wi-Fi Aware events
		wifiAwareLibrary.addCallback(this);
		
		Log.d(TAG, "NetworkInitializer created with UUID: " + myUUID);
	}

	public void addListener(NetworkConnectionListener listener) {
		listeners.add(listener);
	}

	public void removeListener(NetworkConnectionListener listener) {
		listeners.remove(listener);
	}

	public void initialize() {
		if (isInitialized) {
			Log.d(TAG, "Already initialized");
			return;
		}

		if (!wifiAwareLibrary.isWifiAwareSupported()) {
			String error = "Wi-Fi Aware not supported on this device";
			Log.e(TAG, error);
			notifyError(error);
			return;
		}

		if (!wifiAwareLibrary.isWifiAwareAvailable()) {
			String error = "Wi-Fi Aware not currently available";
			Log.w(TAG, error);
			notifyError(error);
			return;
		}

		Log.d(TAG, "Initializing Wi-Fi Aware...");
		wifiAwareLibrary.initialize();
	}

	public void startPublishing() {
		if (!isInitialized) {
			Log.w(TAG, "Not initialized, cannot start publishing");
			return;
		}

		Log.d(TAG, "Starting Wi-Fi Aware publishing...");
		wifiAwareLibrary.startPublishing();
	}

	public void startSubscribing() {
		if (!isInitialized) {
			Log.w(TAG, "Not initialized, cannot start subscribing");
			return;
		}

		Log.d(TAG, "Starting Wi-Fi Aware subscribing...");
		wifiAwareLibrary.startSubscribing();
	}

	public void startBoth() {
		startPublishing();
		startSubscribing();
	}

	public Map<PeerHandle, DataPathInfo> getActiveConnections() {
		return wifiAwareLibrary.getActiveConnections();
	}
	
	public Map<PeerHandle, DataPathInfo> getAllPeers() {
		return wifiAwareLibrary.getAllPeers();
	}
	
	public WifiAwareLibrary getWifiAwareLibrary() {
		return wifiAwareLibrary;
	}

	public String getWifiAwareStats() {
		return wifiAwareLibrary.getWifiAwareStats();
	}

	public boolean isWifiAwareAvailable() {
		return wifiAwareLibrary.isWifiAwareAvailable();
	}

	public boolean isInitialized() {
		return isInitialized;
	}

	public boolean isPublishing() {
		return isPublishing;
	}

	public boolean isSubscribing() {
		return isSubscribing;
	}

	public UUID getDeviceUUID() {
		return myUUID;
	}

	// WifiAwareLibrary.WifiAwareCallback implementation
	@Override
	public void onWifiAwareAvailable(boolean available) {
		Log.d(TAG, "Wi-Fi Aware availability changed: " + available);
		
		for (NetworkConnectionListener listener : listeners) {
			listener.onWifiAwareStateChanged(available);
		}
		
		if (!available && isInitialized) {
			// Clean up when Wi-Fi Aware becomes unavailable
			isInitialized = false;
			isPublishing = false;
			isSubscribing = false;
		}
	}

	@Override
	public void onSessionAttached(WifiAwareSession session) {
		Log.d(TAG, "Wi-Fi Aware session attached successfully");
		isInitialized = true;
		
		// Automatically start both publishing and subscribing
		startBoth();
		
		// Notify session info change
		notifySessionInfoChanged();
	}

	@Override
	public void onPublishStarted(PublishDiscoverySession session) {
		Log.d(TAG, "Wi-Fi Aware publishing started");
		isPublishing = true;
		notifySessionInfoChanged();
	}

	@Override
	public void onSubscribeStarted(SubscribeDiscoverySession session) {
		Log.d(TAG, "Wi-Fi Aware subscribing started");
		isSubscribing = true;
		notifySessionInfoChanged();
	}

	@Override
	public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo) {
		Log.d(TAG, "Wi-Fi Aware service discovered: " + peerHandle);
		
		// Parse service specific info if needed
		String info = new String(serviceSpecificInfo);
		Log.d(TAG, "Service info: " + info);
		
		// Notify listeners about peer discovery
		DataPathInfo peerInfo = new DataPathInfo(peerHandle);
		for (NetworkConnectionListener listener : listeners) {
			listener.onPeerDiscovered(peerHandle, peerInfo);
		}
	}

	@Override
	public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
		String messageStr = new String(message);
		Log.d(TAG, "Message received from " + peerHandle + ": " + messageStr);
	}

	@Override
	public void onConnectionEstablished(PeerHandle peerHandle, DataPathInfo dataPath) {
		Log.d(TAG, "Wi-Fi Aware connection established: " + peerHandle + " -> " + dataPath.ipAddress);
		
		for (NetworkConnectionListener listener : listeners) {
			listener.onConnectionEstablished(dataPath.ipAddress, peerHandle);
		}
	}

	@Override
	public void onConnectionLost(PeerHandle peerHandle) {
		Log.d(TAG, "Wi-Fi Aware connection lost: " + peerHandle);
		
		for (NetworkConnectionListener listener : listeners) {
			listener.onConnectionLost("", peerHandle);
		}
	}

	@Override
	public void onPeerDiscovered(PeerHandle peerHandle) {
		Log.d(TAG, "Peer discovered: " + peerHandle);
		
		// Create peer info and notify listeners
		DataPathInfo peerInfo = new DataPathInfo(peerHandle);
		for (NetworkConnectionListener listener : listeners) {
			listener.onPeerDiscovered(peerHandle, peerInfo);
		}
	}

	@Override
	public void onPeerLost(PeerHandle peerHandle) {
		Log.d(TAG, "Peer lost: " + peerHandle);
		
		// Notify listeners about peer loss
		for (NetworkConnectionListener listener : listeners) {
			listener.onPeerLost(peerHandle);
		}
	}

	@Override
	public void onRoleChanged(DeviceRole newRole) {
		Log.d(TAG, "Device role changed to: " + newRole);
		
		// Notify listeners about role change
		for (NetworkConnectionListener listener : listeners) {
			listener.onRoleChanged(newRole);
		}
		
		// Also notify about session info change
		if (wifiAwareLibrary != null) {
			WifiAwareModels.SessionInfo sessionInfo = wifiAwareLibrary.getSessionInfo();
			for (NetworkConnectionListener listener : listeners) {
				listener.onSessionInfoChanged(sessionInfo);
			}
		}
	}

	

	@Override
	public void onError(String error) {
		Log.e(TAG, "Wi-Fi Aware error: " + error);
		notifyError(error);
	}

	private void notifyError(String error) {
		for (NetworkConnectionListener listener : listeners) {
			listener.onError(error);
		}
	}
	
	private void notifySessionInfoChanged() {
		if (wifiAwareLibrary != null) {
			WifiAwareModels.SessionInfo sessionInfo = wifiAwareLibrary.getSessionInfo();
			for (NetworkConnectionListener listener : listeners) {
				listener.onSessionInfoChanged(sessionInfo);
			}
		}
	}
	
	public void clearConnectionLogs() {
		if (wifiAwareLibrary != null) {
			wifiAwareLibrary.clearConnectionLogs();
		}
	}
	
	public List<String> getConnectionLogs() {
		if (wifiAwareLibrary != null) {
			return wifiAwareLibrary.getConnectionLogs();
		}
		return new ArrayList<>();
	}

	public void cleanup() {
		Log.d(TAG, "Cleaning up NetworkInitializer");
		
		isInitialized = false;
		isPublishing = false;
		isSubscribing = false;
		
		if (wifiAwareLibrary != null) {
			wifiAwareLibrary.cleanup();
		}
		
		listeners.clear();
	}

	public void destroy() {
		cleanup();
		
		if (wifiAwareLibrary != null) {
			wifiAwareLibrary.destroy();
		}
	}
}
