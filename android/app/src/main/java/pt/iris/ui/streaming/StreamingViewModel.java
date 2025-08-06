package pt.iris.ui.streaming;

import android.content.Context;
import android.net.Uri;
import android.net.wifi.aware.PeerHandle;
import android.os.ParcelFileDescriptor;
import android.util.Log;
import androidx.annotation.OptIn;
import androidx.lifecycle.LiveData;
import androidx.lifecycle.MutableLiveData;
import androidx.lifecycle.ViewModel;
import androidx.media3.common.MediaItem;
import androidx.media3.common.util.UnstableApi;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory;
import pt.iris.services.RustService;
import pt.iris.network.NetworkInitializer;
import pt.iris.network.lib.WifiAwareModels;
import pt.iris.network.lib.WifiAwareModels.DataPathInfo;
import pt.iris.network.lib.WifiAwareModels.SessionInfo;
import pt.iris.utils.AppContainer;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Timer;
import java.util.TimerTask;
import java.util.concurrent.ConcurrentHashMap;

public class StreamingViewModel extends ViewModel implements NetworkInitializer.NetworkConnectionListener {
	// [mine] DEBUG: Log every function entry for UI update tracing

	private final MutableLiveData<String> mText;
	private final MutableLiveData<NetworkStats> networkStats;
	private final MutableLiveData<Boolean> isPlayerReady;
	private final MutableLiveData<Boolean> isStreamConnected;
	private final MutableLiveData<Map<PeerHandle, DataPathInfo>> wifiAwareConnections;
	private final MutableLiveData<SessionInfo> wifiAwareSessionInfo;
	private final MutableLiveData<List<String>> connectionLogs;
	// Add this field to track peer roles
	private Map<PeerHandle, WifiAwareModels.DeviceRole> peerRoles = new ConcurrentHashMap<>();

	private ExoPlayer player;
	private ParcelFileDescriptor[] pipe;
	private Timer statsTimer;
	private NetworkInitializer networkInitializer;
	private Uri currentStreamUri;

	private final String TAG = "StreamingViewModel";

	public static class NetworkStats {
		public long totalBytesReceived;
		public long uptime;
		public double currentBitrate;
		public int packetsLost;
		public double packetLossRate;
		public String wifiAwareDataPaths;

		public NetworkStats() {
			this.totalBytesReceived = 0;
			this.uptime = 0;
			this.currentBitrate = 0.0;
			this.packetsLost = 0;
			this.packetLossRate = 0.0;
			this.wifiAwareDataPaths = "N/A";
		}
	}

	public StreamingViewModel() {
		mText = new MutableLiveData<>();
		mText.setValue("WiFi Aware Video Streaming");
		
		networkStats = new MutableLiveData<>();
		networkStats.setValue(new NetworkStats());
		
		isPlayerReady = new MutableLiveData<>();
		isPlayerReady.setValue(false);
		
		isStreamConnected = new MutableLiveData<>();
		isStreamConnected.setValue(false);
		
		wifiAwareConnections = new MutableLiveData<>();
		wifiAwareConnections.setValue(new ConcurrentHashMap<>());
		
		wifiAwareSessionInfo = new MutableLiveData<>();
		
		connectionLogs = new MutableLiveData<>();
		connectionLogs.setValue(new ArrayList<>());
		
		// Initialize WiFi Aware network initializer
		try {
			networkInitializer = AppContainer.getInstance().getNetworkInitializer();
			if (networkInitializer != null) {
				networkInitializer.addListener(this);
			}
		} catch (Exception e) {
			// Handle case where AppContainer might not be initialized yet
			networkInitializer = null;
		}
		
		// Start auto-refresh stats collection immediately
		startStatsCollection();
	}

	@OptIn(markerClass = UnstableApi.class)
	public ExoPlayer initializePlayer(Context context, Uri streamUri) {
		if (player != null) {
			releasePlayer(); // Release existing player first
		}

		try {
			// Create pipe for data transfer
			pipe = ParcelFileDescriptor.createPipe();
			ParcelFileDescriptor readPfd = pipe[0];
			ParcelFileDescriptor writePfd = pipe[1];

			// Extract stream URL from URI for Rust client
			String streamUrl = extractStreamUrlFromUri(streamUri);

			// Start native client with the stream URL and check if connection succeeded
			boolean connectionSuccessful = RustService.startClient(writePfd.getFd(), streamUrl);

			if (connectionSuccessful) {
				android.util.Log.i(TAG, "Client connected successfully, triggering WiFi Aware initialization");
				isStreamConnected.postValue(true); // Update UI immediately on success
				try {
					pt.iris.utils.AppContainer.getInstance().triggerNetworkInitialization();
				} catch (Exception e) {
					android.util.Log.w(TAG, "Failed to trigger network initialization: " + e.getMessage());
				}
			} else {
				android.util.Log.e(TAG, "Failed to connect to server");
				isStreamConnected.postValue(false); // Ensure UI is updated on failure
				throw new RuntimeException("Failed to connect to server");
			}

			// Create ExoPlayer with custom data source
			player = new ExoPlayer.Builder(context)
					.setMediaSourceFactory(new DefaultMediaSourceFactory(new PfdPipeDataSource.Factory(readPfd)))
					.build();

			// Set up media item with the provided URI
			MediaItem mediaItem = MediaItem.fromUri(streamUri);
			player.setMediaItem(mediaItem);
			player.prepare();

			// Add listener for player state
			player.addListener(new androidx.media3.common.Player.Listener() {
				@Override
				public void onPlaybackStateChanged(int playbackState) {
					isPlayerReady.setValue(playbackState == androidx.media3.common.Player.STATE_READY);
				}
			});

			startStatsCollection();

		} catch (IOException e) {
			isStreamConnected.postValue(false); // Update UI on player init failure
			throw new RuntimeException("Failed to initialize player", e);
		}

		return player;
	}

	private void startStatsCollection() {
		if (statsTimer != null) {
			statsTimer.cancel();
			statsTimer = null;
		}
		
		// Use daemon timer so it doesn't prevent JVM shutdown
		statsTimer = new Timer("StreamingViewModel-StatsTimer", true);
		statsTimer.schedule(new TimerTask() {
			@Override
			public void run() {
				updateNetworkStats();
			}
		}, 0, 1000); // Update every 1 second (auto-refresh)
	}

	private void updateNetworkStats() {
		NetworkStats stats = networkStats.getValue();

		// Update Wi-Fi Aware specific stats
		if (networkInitializer != null) {
			stats.wifiAwareDataPaths = networkInitializer.getWifiAwareLibrary().wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount() + "/" + networkInitializer.getWifiAwareLibrary().wifiAwareManager.getCharacteristics().getNumberOfSupportedDataPaths();
			// Update session info
			updateSessionInfo();
		}

		// Grab packet loss stats from Rust client
		assert stats != null;
		stats.packetLossRate =  RustService.getPacketLossPercentage();

		// Update bitrate and total bytes received from PfdPipeDataSource
		try {
			pt.iris.ui.streaming.PfdPipeDataSource pfdSource = pt.iris.ui.streaming.PfdPipeDataSource.Factory.getLastInstance();
			if (pfdSource != null) {
				long totalBytes = pfdSource.getTotalBytesRead();
				stats.totalBytesReceived = totalBytes;
				// Calculate bitrate (bits per second)
				long uptime = stats.uptime > 0 ? stats.uptime : 1;
				stats.currentBitrate = (totalBytes * 8.0) / uptime;
			}
		} catch (Exception ignored) {}

		// Update uptime
		stats.uptime += 1; // Called every second

		networkStats.postValue(stats);
	}

	// Getters for LiveData

	public LiveData<NetworkStats> getNetworkStats() {
		return networkStats;
	}

	public LiveData<Boolean> getIsPlayerReady() {
		return isPlayerReady;
	}

	public LiveData<Boolean> getIsStreamConnected() {
		return isStreamConnected;
	}
	
	public LiveData<Map<PeerHandle, DataPathInfo>> getWifiAwareConnections() {
		return wifiAwareConnections;
	}
	
	public LiveData<SessionInfo> getWifiAwareSessionInfo() {
		return wifiAwareSessionInfo;
	}

	public ExoPlayer getPlayer() {
		return player;
	}


	@Override
	public void onConnectionEstablished(String ipAddress, PeerHandle peerHandle) {
		Log.d(TAG, "Wi-Fi Aware connection established: " + ipAddress);
		Map<PeerHandle, DataPathInfo> allPeers = networkInitializer.getAllPeers();
		Log.d(TAG, "All peers map size (after connection): " + (allPeers != null ? allPeers.size() : 0));
		if (allPeers != null) {
			// Use the actual DataPathInfo from activeConnections if available
			Map<PeerHandle, DataPathInfo> activeConnections = networkInitializer.getWifiAwareLibrary().getActiveConnections();
			DataPathInfo connectedInfo = activeConnections.get(peerHandle);
			if (connectedInfo != null) {
				connectedInfo.isConnected = true;
				connectedInfo.isActive = true;
				allPeers.put(peerHandle, connectedInfo);
				wifiAwareConnections.postValue(allPeers);

			} else {
				Log.w(TAG, "No DataPathInfo found in activeConnections for peer: " + peerHandle);
			}


		}
	}

	@Override
	public void onConnectionLost(String ipAddress, PeerHandle peerHandle) {
		Log.d(TAG, "Wi-Fi Aware connection lost: " + ipAddress);
		Map<PeerHandle, DataPathInfo> allPeers = networkInitializer.getAllPeers();
		Log.d(TAG, "All peers map size (after lost): " + (allPeers != null ? allPeers.size() : 0));
		if (allPeers != null) {
			wifiAwareConnections.postValue(new ConcurrentHashMap<>(allPeers));
		}
	}

	@Override
	public void onWifiAwareStateChanged(boolean available) {
		Log.d(TAG, "Wi-Fi Aware state changed: " + available);
		if (!available) {
			// Clear connections when Wi-Fi Aware becomes unavailable
			wifiAwareConnections.postValue(new ConcurrentHashMap<>());
		}
	}


	@Override
	public void onError(String error) {
		Log.e(TAG, "Wi-Fi Aware error: " + error);
	}

	// Discovered peers are not shown in the UI

	/**
	 * Connect to a discovered peer by PeerHandle (manual connect from UI)
	 */
	public void connectToPeer(PeerHandle peerHandle) {
		if (networkInitializer != null) {
			// Always try as master initiator for simplicity (or add role logic if needed)
			networkInitializer.getWifiAwareLibrary().establishDataPath(peerHandle, true);
			Log.d(TAG, "Requested connection to peer: " + peerHandle);
		} else {
			Log.w(TAG, "NetworkInitializer is null, cannot connect to peer");
		}
	}

	// Add this method to expose peer roles to the UI and force UI update when changed
	public Map<PeerHandle, WifiAwareModels.DeviceRole> getPeerRoles() {
		if (networkInitializer != null && networkInitializer.getWifiAwareLibrary() != null) {
			// Get the up-to-date peerRoles map from the library
			try {
				java.lang.reflect.Field field = networkInitializer.getWifiAwareLibrary().getClass().getDeclaredField("peerRoles");
				field.setAccessible(true);
				Object value = field.get(networkInitializer.getWifiAwareLibrary());
				if (value instanceof Map) {
					//noinspection unchecked
					peerRoles = new ConcurrentHashMap<>((Map<PeerHandle, WifiAwareModels.DeviceRole>) value);
				}
			} catch (Exception e) {
				// fallback: keep previous peerRoles
			}
		}
		// Do NOT force UI update here to avoid infinite loop
		return peerRoles;
	}

	@Override
	public void onRoleChanged(WifiAwareModels.DeviceRole newRole) {
		Log.d(TAG, "Role changed to: " + newRole);
		updateSessionInfo();
		// Force UI update for peer role change
		if (wifiAwareConnections != null && wifiAwareConnections.getValue() != null) {
			wifiAwareConnections.postValue(new ConcurrentHashMap<>(wifiAwareConnections.getValue()));
		}
	}

	@Override
	public void onSessionInfoChanged(SessionInfo sessionInfo) {
		Log.d(TAG, "Session info changed");
		wifiAwareSessionInfo.postValue(sessionInfo);
	}

	public void connectToStream(Context context, Uri streamUri) {
		// Initialize and configure the player with the provided URI
		player = initializePlayer(context, streamUri);
		currentStreamUri = streamUri;
		
		// Start playback
		player.play();
		isStreamConnected.setValue(true);
	}

	public void disconnectStream() {
		if (player != null) {
			player.stop();
			player.clearMediaItems();

			releasePlayer();
			currentStreamUri = null;
			isStreamConnected.setValue(false);
			isPlayerReady.setValue(false);


		}
	}

	private void releasePlayer() {
		// Stop and cancel timer first
		if (statsTimer != null) {
			statsTimer.cancel();
			statsTimer = null;
		}
		
		// Stop Rust client
		try {
			RustService.stopClient();
		} catch (Exception e) {
			android.util.Log.w(TAG, "Error stopping Rust client", e);
		}
		
		// Release ExoPlayer
		if (player != null) {
			player.release();
			player = null;
		}


	}

	@Override
	protected void onCleared() {
		super.onCleared();
		
		// Remove network connection listener
		if (networkInitializer != null) {
			networkInitializer.removeListener(this);
		}

		releasePlayer();
	}

	/**
	 * Extract a stream URL from URI for the Rust client
	 * Converts RTSP URIs to host names for iris_client
	 */
	private String extractStreamUrlFromUri(Uri uri) {
		if (uri == null) {
			return "stream1.iris.dcc.fc.up.pt"; // Default fallback
		}
		
		String host = uri.getHost();
		if (host != null && !host.isEmpty()) {
			return host;
		}
		
		// Try to extract from the full URI string
		String uriString = uri.toString();
		if (uriString.startsWith("iris://")) {
			// Extract host from RTSP URL
			try {
				String withoutProtocol = uriString.substring(7); // Remove "iris://"
				int slashIndex = withoutProtocol.indexOf('/');
				int colonIndex = withoutProtocol.indexOf(':');
				
				int endIndex = withoutProtocol.length();
				if (slashIndex != -1) endIndex = Math.min(endIndex, slashIndex);
				if (colonIndex != -1) endIndex = Math.min(endIndex, colonIndex);
				
				String extractedHost = withoutProtocol.substring(0, endIndex);
				if (!extractedHost.isEmpty()) {
					return extractedHost;
				}
			} catch (Exception e) {
				android.util.Log.w(TAG, "Failed to extract host from URI: " + uriString, e);
			}
		}
		
		// Final fallback
		return "stream1.iris.dcc.fc.up.pt";
	}
	
	
	private void updateSessionInfo() {
		if (networkInitializer != null && networkInitializer.getWifiAwareLibrary() != null) {
			SessionInfo sessionInfo = networkInitializer.getWifiAwareLibrary().getSessionInfo();
			wifiAwareSessionInfo.postValue(sessionInfo);
		}
	}

	/**
	 * Stop streaming and release all resources
	 */
	public void stopStreaming() {
		releasePlayer();
	}

}
