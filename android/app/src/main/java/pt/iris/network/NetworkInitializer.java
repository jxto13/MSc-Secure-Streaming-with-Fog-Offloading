
package pt.iris.network;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareSession;
import android.util.Log;

import java.nio.charset.StandardCharsets;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CopyOnWriteArrayList;

import pt.iris.network.lib.WifiAwareLibrary;
import pt.iris.network.lib.WifiAwareModels;
import pt.iris.network.lib.WifiAwareModels.DataPathInfo;
import pt.iris.network.lib.WifiAwareModels.DeviceRole;

@SuppressLint("MissingPermission")
public class NetworkInitializer implements WifiAwareLibrary.WifiAwareCallback {
	// [mine] DEBUG: Log every function entry for UI update tracing
	private static final String TAG = "NetworkInitializer";

	private final Context context;
	private final WifiAwareLibrary wifiAwareLibrary;
	private final CopyOnWriteArrayList<NetworkConnectionListener> listeners = new CopyOnWriteArrayList<>();
	private UUID myUUID;

	// Only show active connections in the UI

	private boolean isInitialized = false;
	private SubscribeDiscoverySession subscribeSession = null;
	private PublishDiscoverySession publishSession = null;

	public interface NetworkConnectionListener {
		void onConnectionEstablished(String ipAddress, PeerHandle peerHandle);
		void onConnectionLost(String ipAddress, PeerHandle peerHandle);
		void onWifiAwareStateChanged(boolean available);
		void onError(String error);
		// Discovered peer callbacks removed from UI
		void onRoleChanged(DeviceRole newRole);
		void onSessionInfoChanged(WifiAwareModels.SessionInfo sessionInfo);
	}

	public NetworkInitializer(Context context) {
		this.context = context;
		this.wifiAwareLibrary = new WifiAwareLibrary(context);
		this.myUUID = UUID.randomUUID();
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
		if (publishSession != null) {
			Log.d(TAG, "Already publishing, skipping duplicate publisher.");

		}
		Log.d(TAG, "Starting Wi-Fi Aware publishing...");
		wifiAwareLibrary.startPublishing();
	}

	public void startSubscribing() {
		if (!isInitialized) {
			Log.w(TAG, "Not initialized, cannot start subscribing");
			return;
		}
		if (subscribeSession != null) {
			Log.d(TAG, "Already subscribing, skipping duplicate subscribe.");
			return;
		}
		Log.d(TAG, "Starting Wi-Fi Aware subscribing...");
		wifiAwareLibrary.startSubscribing();
	}

	public void startBoth() {
		startPublishing();
		startSubscribing();
	}


	/**
	 * Only return active connections for UI.
	 */
	public Map<PeerHandle, DataPathInfo> getAllPeers() {
		return wifiAwareLibrary.getActiveConnections();
	}

	public WifiAwareLibrary getWifiAwareLibrary() {
		return wifiAwareLibrary;
	}

	public String getWifiAwareStats() {
		return wifiAwareLibrary.getWifiAwareStats();
	}


	@Override
	public void onWifiAwareAvailable(boolean available) {
		Log.d(TAG, "Wi-Fi Aware availability changed: " + available);
		for (NetworkConnectionListener listener : listeners) {
			listener.onWifiAwareStateChanged(available);
		}
		if (!available && isInitialized) {
			isInitialized = false;
		}
	}

	@Override
	public void onSessionAttached(WifiAwareSession session) {
		Log.d(TAG, "Wi-Fi Aware session attached successfully");
		isInitialized = true;
		startBoth();
		notifySessionInfoChanged();
	}

	@Override
	public void onPublishStarted(PublishDiscoverySession session) {
		Log.d(TAG, "Wi-Fi Aware publishing started");
		notifySessionInfoChanged();
	}

	@Override
	public void onSubscribeStarted(SubscribeDiscoverySession session) {
		Log.d(TAG, "Wi-Fi Aware subscribing started");
		notifySessionInfoChanged();
	}

	@Override
	public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo) {
		String info = new String(serviceSpecificInfo, StandardCharsets.UTF_8);
		Log.d(TAG, "Wi-Fi Aware service discovered: " + peerHandle  + "Service info: " + info);
	}

	@Override
	public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
		String messageStr = new String(message, StandardCharsets.UTF_8);
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
		String ip = "";
		DataPathInfo info = wifiAwareLibrary.getActiveConnections().get(peerHandle);
		if (info != null) ip = info.ipAddress;
		for (NetworkConnectionListener listener : listeners) {
			listener.onConnectionLost(ip, peerHandle);
		}
	}

	@Override
	public void onPeerDiscovered(PeerHandle peerHandle) {
		Log.d(TAG, "[mine] onPeerDiscovered (NetworkInitializer): " + peerHandle);
	}

	@Override
	public void onPeerLost(PeerHandle peerHandle) {
		Log.d(TAG, "Peer lost: " + peerHandle);
	}

	@Override
	public void onRoleChanged(DeviceRole newRole) {
		Log.d(TAG, "Device role changed to: " + newRole);
		for (NetworkConnectionListener listener : listeners) {
			listener.onRoleChanged(newRole);
		}
		notifySessionInfoChanged();
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

	public void cleanup() {
		Log.d(TAG, "Cleaning up NetworkInitializer");
		isInitialized = false;
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
