package pt.iris.network;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.MacAddress;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.Characteristics;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.IdentityChangedListener;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Build;
import android.util.Log;

import androidx.annotation.NonNull;
import androidx.annotation.RequiresApi;

import pt.iris.network.utils.Utils;
import pt.iris.utils.AppContainer;
import pt.iris.services.RustService;

import java.net.Inet6Address;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;

/**
 * NetworkInitializer - Enhanced WiFi Aware Manager with NAN Data Path Support
 * 
 * This class implements efficient WiFi Aware communication using NAN (Neighbor Aware Networking)
 * data paths instead of creating multiple network interfaces. This approach:
 * 
 * 1. SCALABILITY: Uses data paths (not interfaces) allowing multiple concurrent connections
 * 2. EFFICIENCY: Shares a single WiFi Aware interface for all peer connections  
 * 3. RESOURCE MANAGEMENT: Respects device limits and tracks active connections
 * 4. INTEGRATION: Seamlessly integrates with Rust streaming backend via socket management
 * 
 * Key Improvements:
 * - NAN data paths for direct IPv6 peer-to-peer communication
 * - Proper resource tracking and cleanup
 * - Enhanced error handling and monitoring
 * - Efficient multi-peer support without interface limitations
 */
@SuppressLint("MissingPermission")
public class NetworkInitializer {
	private static final String TAG = "NetworkInitializer";
	public static final String SERVICE_NAME = "Iris_Streaming_Service";

	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	private UUID myUUID;
	private ConnectivityManager connectivityManager;

	// Data path management
	private final Map<PeerHandle, DataPathInfo> activeDataPaths = new ConcurrentHashMap<>();
	private final Map<Network, PeerHandle> networkToPeerMap = new HashMap<>();
	
	// Discovery sessions
	private PublishDiscoverySession publisherSession;
	private SubscribeDiscoverySession subscriberSession;

	private int MAX_INTERFACES = 0;
	private int MAX_DATA_PATHS = 0;

	// Data path information holder
	public static class DataPathInfo {
		public final PeerHandle peerHandle;
		public final Network network;
		public final String ipAddress;
		public final long createdTime;
		public boolean isActive;

		public DataPathInfo(PeerHandle peerHandle, Network network, String ipAddress) {
			this.peerHandle = peerHandle;
			this.network = network;
			this.ipAddress = ipAddress;
			this.createdTime = System.currentTimeMillis();
			this.isActive = true;
		}
	}

	public NetworkInitializer(Context context) {
		this.context = context;
	}

	/**
	 * Entrypoint for the NetworkInitializer
	 */
	@RequiresApi(api = Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
	public void start() {
		this.myUUID = AppContainer.getInstance().getMyUUID();

		// Log UUID for debugging
		Log.d(TAG, "NetworkInitializer starting with UUID: " + myUUID.toString());

		// Get instance of ConnectivityManager
		this.connectivityManager =  (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);

		// Start wifiAwareManager
		startWifiAwareManager();

		// Start WifiAwareSession
		createWifiAwareSession();
	}

	/**
	 * Initialize Wi-Fi Aware Manager with proper data path management
	 */
	@RequiresApi(api = Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
	private void startWifiAwareManager() {
		wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			wifiAwareManagerStatus = true;

			Characteristics characteristics = wifiAwareManager.getCharacteristics();

			Log.d(TAG, "=== WiFi Aware Characteristics ===");
			Log.d(TAG, "Aware Pairing Supported: " + characteristics.isAwarePairingSupported());
			Log.d(TAG, "Instant Communication Mode Supported: " + characteristics.isInstantCommunicationModeSupported());
			Log.d(TAG, "Set Channel On Data Path Supported: " + wifiAwareManager.isSetChannelOnDataPathSupported());
			Log.d(TAG, "Number of Supported Data Interfaces: " + characteristics.getNumberOfSupportedDataInterfaces());
			Log.d(TAG, "Number of Supported Data Paths: " + characteristics.getNumberOfSupportedDataPaths());

			// Get current available resources
			Log.d(TAG, "=== Current Available Resources ===");
			Log.d(TAG, "Available Data Paths Count: " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
			Log.d(TAG, "Available Subscribe Sessions Count: " + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount());
			Log.d(TAG, "Available Publish Sessions Count: " + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());

			MAX_INTERFACES = characteristics.getNumberOfSupportedDataInterfaces();
			MAX_DATA_PATHS = characteristics.getNumberOfSupportedDataPaths();

			Log.d(TAG, "Max Interfaces: " + MAX_INTERFACES + ", Max Data Paths: " + MAX_DATA_PATHS);
			Log.d(TAG, "Supported Cipher Suites: " + characteristics.getSupportedCipherSuites());
		} else {
			// Wi-Fi Aware is not available on this device
			wifiAwareManagerStatus = false;
			Log.e(TAG, "WiFi Aware Manager not available on this device");
		}
	}

	/**
	 * Attach to Wi-Fi Aware and create a session with proper data path management
	 */
	@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
	private void createWifiAwareSession() {
		AppContainer appContainer = AppContainer.getInstance();

		if (wifiAwareManagerStatus) {
			wifiAwareManager.attach(new AttachCallback() {

				@Override
				public void onAttached(WifiAwareSession session) {
					super.onAttached(session);
					mWifiAwareSession = session;
					Log.d(TAG, "WiFi Aware session successfully attached");

					// Auto-start publisher and subscriber for streaming
					startPublisher(session);
					startSubscriber(session);
				}

				@Override
				public void onAttachFailed() {
					super.onAttachFailed();
					Log.e(TAG, "WiFi Aware session attachment failed");
				}

				@Override
				public void onAwareSessionTerminated(){
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
					switch (clusterEventType){
						case CLUSTER_CHANGE_EVENT_STARTED:
							Log.d(TAG, "Cluster STARTED - " + clusterId);
							break;
						case CLUSTER_CHANGE_EVENT_JOINED:
							Log.d(TAG, "Cluster JOINED - " + clusterId);
							break;
					}
				}
			}, null);
		}
	}

	/**
	 * Start publisher with proper data path management
	 */
	@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
	private void startPublisher(WifiAwareSession session) {
		if (publisherSession != null) {
			Log.w(TAG, "Publisher already active");
			return;
		}

		PublishConfig config = new PublishConfig.Builder()
				.setServiceName(SERVICE_NAME)
				.setServiceSpecificInfo(Utils.asBytes(myUUID))
				.setTerminateNotificationEnabled(true)
				.build();

		session.publish(config, new DiscoverySessionCallback() {
			@Override
			public void onPublishStarted(PublishDiscoverySession session) {
				Log.d(TAG, "Publisher started successfully with session ID: " + session.hashCode());
				publisherSession = session;
			}

			@Override
			public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
				Log.d(TAG, "Publisher received message from peer: " + peerHandle);
				establishDataPath(peerHandle, publisherSession, true);
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
	private void startSubscriber(WifiAwareSession session) {
		if (subscriberSession != null) {
			Log.w(TAG, "Subscriber already active");
			return;
		}

		SubscribeConfig config = new SubscribeConfig.Builder()
				.setServiceName(SERVICE_NAME)
				.setTerminateNotificationEnabled(true)
				.build();

		session.subscribe(config, new DiscoverySessionCallback() {
			@Override
			public void onSubscribeStarted(SubscribeDiscoverySession session) {
				Log.d(TAG, "Subscriber started successfully with session ID: " + session.hashCode());
				subscriberSession = session;
			}

			@Override
			public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
				Log.d(TAG, "Service discovered from peer: " + peerHandle);
				// Send initial message to establish communication
				subscriberSession.sendMessage(peerHandle, 0, new byte[0]);
			}

			@Override
			public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
				Log.d(TAG, "Subscriber received message from peer: " + peerHandle);
				establishDataPath(peerHandle, subscriberSession, false);
			}

			@Override
			public void onSessionTerminated() {
				Log.w(TAG, "Subscriber session terminated");
				subscriberSession = null;
			}
		}, null);
	}

	/**
	 * Establish a NAN data path for efficient multi-peer communication
	 * Uses data paths instead of creating new network interfaces
	 */
	private void establishDataPath(PeerHandle peerHandle, DiscoverySession discoverySession, boolean isPublisher) {
		// Check if we already have a data path with this peer
		if (activeDataPaths.containsKey(peerHandle)) {
			Log.d(TAG, "Data path already exists with peer: " + peerHandle);
			return;
		}

		// Check current system resources
		int availableDataPaths = wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount();
		if (availableDataPaths <= 0) {
			Log.w(TAG, "No available data paths from system resources. Cannot create new data path.");
			return;
		}

		// Check if we've reached our internal limit
		if (activeDataPaths.size() >= MAX_DATA_PATHS) {
			Log.w(TAG, "Maximum number of data paths reached (" + MAX_DATA_PATHS + "). Cannot create new data path.");
			return;
		}

		Log.d(TAG, "Establishing NAN data path with peer: " + peerHandle + " (isPublisher: " + isPublisher + 
			", available: " + availableDataPaths + ", active: " + activeDataPaths.size() + ")");

		NetworkSpecifier networkSpecifier;
		if (isPublisher) {
			// Publisher creates the data path with PSK for security
			networkSpecifier = new WifiAwareNetworkSpecifier.Builder(discoverySession, peerHandle)
					.setPskPassphrase(myUUID.toString())
					.setPort(8080) // Set specific port for streaming
					.build();
		} else {
			// Subscriber joins the data path without PSK (publisher will provide it)
			networkSpecifier = new WifiAwareNetworkSpecifier.Builder(discoverySession, peerHandle)
					.setPort(8080) // Set specific port for streaming
					.build();
		}

		NetworkRequest networkRequest = new NetworkRequest.Builder()
				.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
				.setNetworkSpecifier(networkSpecifier)
				.addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) // WiFi Aware is typically not metered
				.build();

		ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
			@Override
			public void onAvailable(Network network) {
				Log.d(TAG, "NAN data path network available: " + network + " for peer: " + peerHandle);
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
				// Log WiFi Aware specific capabilities
				if (networkCapabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI_AWARE)) {
					Log.d(TAG, "Network confirmed as WiFi Aware transport");
				}
			}

			@Override
			public void onLost(Network network) {
				Log.w(TAG, "NAN data path network lost: " + network);
				handleDataPathLost(network);
			}

			@Override
			public void onUnavailable() {
				Log.e(TAG, "NAN data path network unavailable for peer: " + peerHandle);
				// This indicates the data path request failed
			}

			@Override
			public void onBlockedStatusChanged(Network network, boolean blocked) {
				Log.d(TAG, "NAN data path blocked status changed: " + blocked + " for network: " + network);
			}
		};

		// Request the NAN data path
		connectivityManager.requestNetwork(networkRequest, callback);
	}

	/**
	 * Handle when a NAN data path becomes available
	 * This creates a direct IPv6 link between devices without using additional interfaces
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
					// Remove zone identifier if present (e.g., %aware_data0)
					if (ipAddress.contains("%")) {
						ipAddress = ipAddress.substring(0, ipAddress.indexOf("%"));
					}

					// Store the NAN data path information
					DataPathInfo dataPathInfo = new DataPathInfo(peerHandle, network, ipAddress);
					activeDataPaths.put(peerHandle, dataPathInfo);
					networkToPeerMap.put(network, peerHandle);

					Log.d(TAG, "NAN data path established with peer " + peerHandle + 
						" at IPv6: " + ipAddress + " (interface: " + linkProperties.getInterfaceName() + ")");
					
					// Add this socket to the Rust layer for streaming communication
					RustService.addSocket(ipAddress);
					
					// Log updated statistics
					logDataPathStats();
				} else {
					Log.e(TAG, "No valid IPv6 address found for NAN data path");
				}
			} else {
				Log.e(TAG, "No link properties or addresses available for NAN data path");
			}
		} catch (Exception e) {
			Log.e(TAG, "Error handling NAN data path availability", e);
		}
	}

	/**
	 * Update data path information when link properties change
	 */
	private void updateDataPathInfo(Network network, android.net.LinkProperties linkProperties) {
		PeerHandle peerHandle = networkToPeerMap.get(network);
		if (peerHandle != null && activeDataPaths.containsKey(peerHandle)) {
			DataPathInfo dataPathInfo = activeDataPaths.get(peerHandle);
			if (dataPathInfo != null) {
				Log.d(TAG, "Updated link properties for peer: " + peerHandle);
				// You can update additional information here if needed
			}
		}
	}

	/**
	 * Handle when a NAN data path is lost
	 * This properly cleans up the data path without affecting other connections
	 */
	private void handleDataPathLost(Network network) {
		PeerHandle peerHandle = networkToPeerMap.remove(network);
		if (peerHandle != null) {
			DataPathInfo dataPathInfo = activeDataPaths.remove(peerHandle);
			if (dataPathInfo != null) {
				dataPathInfo.isActive = false;
				Log.d(TAG, "NAN data path lost with peer " + peerHandle + 
					" at IPv6: " + dataPathInfo.ipAddress + 
					" (duration: " + (System.currentTimeMillis() - dataPathInfo.createdTime) + "ms)");
				
				// Remove this socket from the Rust layer
				RustService.removeSocket(dataPathInfo.ipAddress);
				
				// Log updated statistics
				logDataPathStats();
			}
		} else {
			Log.w(TAG, "Unknown network lost: " + network);
		}
	}

	/**
	 * Log current NAN data path statistics for monitoring and debugging
	 */
	private void logDataPathStats() {
		int activeCount = activeDataPaths.size();
		int availableDataPaths = wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount();
		int usedDataPaths = MAX_DATA_PATHS - availableDataPaths;
		
		Log.d(TAG, "=== NAN Data Path Statistics ===");
		Log.d(TAG, "Active NAN data paths: " + activeCount + "/" + MAX_DATA_PATHS);
		Log.d(TAG, "System available data paths: " + availableDataPaths);
		Log.d(TAG, "System used data paths: " + usedDataPaths);
		Log.d(TAG, "Interface limit: " + MAX_INTERFACES + " (not used - using NAN data paths)");
		
		if (activeCount > 0) {
			Log.d(TAG, "--- Active Data Path Details ---");
			for (Map.Entry<PeerHandle, DataPathInfo> entry : activeDataPaths.entrySet()) {
				DataPathInfo info = entry.getValue();
				long ageSeconds = (System.currentTimeMillis() - info.createdTime) / 1000;
				Log.d(TAG, "Peer " + entry.getKey() + " -> IPv6: " + info.ipAddress + 
						", Active: " + info.isActive + ", Age: " + ageSeconds + "s");
			}
		}
		
		// Warn if approaching limits
		if (activeCount >= MAX_DATA_PATHS * 0.8) {
			Log.w(TAG, "Approaching maximum data path limit: " + activeCount + "/" + MAX_DATA_PATHS);
		}
	}

	/**
	 * Stop publisher and clean up associated data paths
	 */
	private void stopPublisher() {
		if (publisherSession != null) {
			publisherSession.close();
			publisherSession = null;
			Log.d(TAG, "Publisher stopped");
		}
	}

	/**
	 * Stop subscriber and clean up associated data paths
	 */
	private void stopSubscriber() {
		if (subscriberSession != null) {
			subscriberSession.close();
			subscriberSession = null;
			Log.d(TAG, "Subscriber stopped");
		}
	}

	/**
	 * Clean up all active NAN data paths efficiently
	 */
	private void cleanupAllDataPaths() {
		Log.d(TAG, "Cleaning up all " + activeDataPaths.size() + " NAN data paths");
		
		// Remove all sockets from Rust layer first
		for (DataPathInfo dataPathInfo : activeDataPaths.values()) {
			if (dataPathInfo.isActive) {
				RustService.removeSocket(dataPathInfo.ipAddress);
				dataPathInfo.isActive = false;
			}
		}
		
		// Clear all tracking maps
		activeDataPaths.clear();
		networkToPeerMap.clear();
		
		Log.d(TAG, "All NAN data paths cleaned up successfully");
	}

	/**
	 * Get information about active data paths
	 */
	public Map<PeerHandle, DataPathInfo> getActiveDataPaths() {
		return new HashMap<>(activeDataPaths);
	}

	/**
	 * Get current NAN data path statistics for UI display
	 */
	public String getDataPathStatistics() {
		if (!wifiAwareManagerStatus) {
			return "WiFi Aware not available";
		}
		
		int activeCount = activeDataPaths.size();
		int availableDataPaths = wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount();
		
		return String.format("NAN Paths: %d/%d active, %d available", 
			activeCount, MAX_DATA_PATHS, availableDataPaths);
	}

	/**
	 * Get detailed statistics for monitoring
	 */
	public String getDetailedStatistics() {
		if (!wifiAwareManagerStatus) {
			return "WiFi Aware Manager: Not Available";
		}

		StringBuilder stats = new StringBuilder();
		stats.append("=== WiFi Aware Status ===\n");
		stats.append("Session Active: ").append(mWifiAwareSession != null).append("\n");
		stats.append("Publisher Active: ").append(publisherSession != null).append("\n");
		stats.append("Subscriber Active: ").append(subscriberSession != null).append("\n");
		stats.append("\n=== NAN Data Paths ===\n");
		stats.append("Max Data Paths: ").append(MAX_DATA_PATHS).append("\n");
		stats.append("Active Data Paths: ").append(activeDataPaths.size()).append("\n");
		stats.append("Available from System: ").append(wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount()).append("\n");
		
		if (!activeDataPaths.isEmpty()) {
			stats.append("\n=== Active Connections ===\n");
			for (Map.Entry<PeerHandle, DataPathInfo> entry : activeDataPaths.entrySet()) {
				DataPathInfo info = entry.getValue();
				long ageSeconds = (System.currentTimeMillis() - info.createdTime) / 1000;
				stats.append("Peer: ").append(entry.getKey().toString().substring(0, Math.min(8, entry.getKey().toString().length())))
					.append("... -> ").append(info.ipAddress)
					.append(" (").append(ageSeconds).append("s)\n");
			}
		}
		
		return stats.toString();
	}

	/**
	 * Check if we can create more data paths
	 */
	public boolean canCreateMoreDataPaths() {
		if (!wifiAwareManagerStatus) return false;
		
		int availableFromSystem = wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount();
		boolean belowInternalLimit = activeDataPaths.size() < MAX_DATA_PATHS;
		
		return availableFromSystem > 0 && belowInternalLimit;
	}

	/**
	 * Force cleanup of a specific data path (for error recovery)
	 */
	public boolean forceCleanupDataPath(String peerHandleStr) {
		for (Map.Entry<PeerHandle, DataPathInfo> entry : activeDataPaths.entrySet()) {
			if (entry.getKey().toString().equals(peerHandleStr)) {
				PeerHandle peerHandle = entry.getKey();
				DataPathInfo dataPathInfo = entry.getValue();
				
				Log.w(TAG, "Force cleaning up data path for peer: " + peerHandle);
				
				// Remove from Rust layer
				RustService.removeSocket(dataPathInfo.ipAddress);
				
				// Remove from our tracking
				activeDataPaths.remove(peerHandle);
				networkToPeerMap.remove(dataPathInfo.network);
				
				logDataPathStats();
				return true;
			}
		}
		return false;
	}

	/**
	 * Get active WiFi Aware data path connections for UI display
	 */
	public List<WifiAwareDataPathManager.WifiAwareConnectionInfo> getActiveConnections() {
		List<WifiAwareDataPathManager.WifiAwareConnectionInfo> connections = new ArrayList<>();
		
		for (Map.Entry<PeerHandle, DataPathInfo> entry : activeDataPaths.entrySet()) {
			DataPathInfo dataPath = entry.getValue();
			if (dataPath.isActive) {
				// Convert our DataPathInfo to the format expected by the UI
				WifiAwareDataPathManager.WifiAwareConnectionInfo connectionInfo = 
					new WifiAwareDataPathManager.WifiAwareConnectionInfo(
						dataPath.peerHandle.toString(), 
						dataPath.ipAddress, 
						dataPath.createdTime,
						dataPath.isActive
					);
				connections.add(connectionInfo);
			}
		}
		
		return connections;
	}





	/**
	 * Get the number of active data paths
	 */
	public int getActiveDataPathCount() {
		int count = 0;
		for (DataPathInfo dataPath : activeDataPaths.values()) {
			if (dataPath.isActive) {
				count++;
			}
		}
		return count;
	}

	/**
	 * Check if WiFi Aware is available and active
	 */
	public boolean isWifiAwareActive() {
		return wifiAwareManagerStatus && mWifiAwareSession != null;
	}

	/**
	 * Force refresh of connections (used by UI refresh buttons)
	 */
	public void forceRefreshConnections() {
		// Log current state for debugging
		Log.d(TAG, "Force refresh: " + getDataPathStatistics());
		
		// Clean up any inactive data paths
		cleanupInactiveDataPaths();
	}

	/**
	 * Clean up inactive data paths
	 */
	public void cleanupInactiveDataPaths() {
		List<PeerHandle> toRemove = new ArrayList<>();
		
		for (Map.Entry<PeerHandle, DataPathInfo> entry : activeDataPaths.entrySet()) {
			DataPathInfo dataPath = entry.getValue();
			
			// Check if data path is too old or marked as inactive
			long ageMs = System.currentTimeMillis() - dataPath.createdTime;
			if (!dataPath.isActive || ageMs > 300000) { // 5 minutes timeout
				toRemove.add(entry.getKey());
			}
		}
		
		for (PeerHandle peerHandle : toRemove) {
			DataPathInfo dataPath = activeDataPaths.remove(peerHandle);
			if (dataPath != null) {
				networkToPeerMap.remove(dataPath.network);
				Log.d(TAG, "Cleaned up inactive data path for peer: " + peerHandle);
			}
		}
	}

	/**
	 * Stops Wi-Fi Aware and destroys all resources created by it
	 */
	public void closeSessions(){
		Log.d(TAG, "Closing all WiFi Aware sessions");
		
		// Stop discovery sessions
		stopPublisher();
		stopSubscriber();
		
		// Clean up all data paths
		cleanupAllDataPaths();
		
		// Close the main WiFi Aware session
		if(mWifiAwareSession != null){
			mWifiAwareSession.close();
			mWifiAwareSession = null;
			Log.d(TAG, "WiFi Aware session closed");
		}
	}
}
