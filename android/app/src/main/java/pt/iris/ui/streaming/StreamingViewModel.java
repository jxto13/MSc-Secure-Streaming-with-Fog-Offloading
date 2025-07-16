package pt.iris.ui.streaming;

import android.content.Context;
import android.content.Intent;
import android.net.Uri;
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
import pt.iris.utils.AppContainer;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Timer;
import java.util.TimerTask;

public class StreamingViewModel extends ViewModel {

	private final MutableLiveData<String> mText;
	private final MutableLiveData<List<WifiAwareClientInfo>> connectedClients;
	private final MutableLiveData<List<WifiAwareModels.ConnectionInfo>> wifiAwareConnections;
	private final MutableLiveData<NetworkStats> networkStats;
	private final MutableLiveData<Boolean> isPlayerReady;
	private final MutableLiveData<Boolean> isStreamConnected;
	
	private ExoPlayer player;
	private ParcelFileDescriptor[] pipe;
	private Timer statsTimer;
	private NetworkInitializer networkInitializer;
	private Uri currentStreamUri;

	public static class WifiAwareClientInfo {
		public String ipAddress;
		public String macAddress;
		public long lastSeen;
		public long bytesReceived;
		public String connectionStatus;

		public WifiAwareClientInfo(String ipAddress, String macAddress) {
			this.ipAddress = ipAddress;
			this.macAddress = macAddress;
			this.lastSeen = System.currentTimeMillis();
			this.bytesReceived = 0;
			this.connectionStatus = "Connected";
		}
	}

	public static class NetworkStats {
		public int totalConnectedClients;
		public long totalBytesReceived;
		public long uptime;
		public double currentBitrate;
		public int packetsLost;
		public double packetLossRate;
		public String wifiAwareDataPaths; // New field for WiFi Aware specific stats

		public NetworkStats() {
			this.totalConnectedClients = 0;
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
		
		connectedClients = new MutableLiveData<>();
		connectedClients.setValue(new ArrayList<>());
		
		wifiAwareConnections = new MutableLiveData<>();
		wifiAwareConnections.setValue(new ArrayList<>());
		
		networkStats = new MutableLiveData<>();
		networkStats.setValue(new NetworkStats());
		
		isPlayerReady = new MutableLiveData<>();
		isPlayerReady.setValue(false);
		
		isStreamConnected = new MutableLiveData<>();
		isStreamConnected.setValue(false);
		
		// Initialize WiFi Aware network initializer
		try {
			networkInitializer = AppContainer.getInstance().getNetworkInitializer();
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
			
			// Start native client with the stream URL
			RustService.startClient(writePfd.getFd(), streamUrl);

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
			throw new RuntimeException("Failed to initialize player", e);
		}

		return player;
	}

	private void startStatsCollection() {
		if (statsTimer != null) {
			statsTimer.cancel();
		}
		
		statsTimer = new Timer();
		statsTimer.scheduleAtFixedRate(new TimerTask() {
			@Override
			public void run() {
				updateNetworkStats();
				updateConnectedClients();
			}
		}, 0, 1000); // Update every 1 second (auto-refresh)
	}

	private void updateNetworkStats() {
		NetworkStats stats = networkStats.getValue();
		if (stats == null) stats = new NetworkStats();
		
		// Get real WiFi Aware connection count from NetworkInitializer
		List<WifiAwareModels.ConnectionInfo> wifiAwareConns = 
			wifiAwareConnections.getValue();
		int wifiAwareCount = wifiAwareConns != null ? wifiAwareConns.size() : 0;
		
		// Use primarily real WiFi Aware connections for stats
		stats.totalConnectedClients = wifiAwareCount;
		
		// Get real stats from NetworkInitializer if available
		if (networkInitializer != null) {
			stats.wifiAwareDataPaths = networkInitializer.getDataPathStatistics();
		}
		
		// Simulate network traffic stats (these would come from Rust layer in real implementation)
		stats.totalBytesReceived += (long)(Math.random() * 1024 * 100); // Random bytes
		stats.currentBitrate = 800000 + (Math.random() * 200000); // 800-1000 kbps
		stats.packetsLost += (int)(Math.random() * 5);
		stats.packetLossRate = (double)stats.packetsLost / (stats.packetsLost + 1000) * 100;
		
		stats.uptime = System.currentTimeMillis() - (System.currentTimeMillis() % 86400000); // Mock uptime
		
		networkStats.postValue(stats);
	}

	private void updateConnectedClients() {
		// Update real WiFi Aware connections from the NetworkInitializer
		if (networkInitializer != null) {
			List<WifiAwareModels.ConnectionInfo> connections = 
				networkInitializer.getActiveConnections();
			wifiAwareConnections.postValue(connections);
		}
	}


	/**
	 * Force refresh of WiFi Aware connections from NetworkInitializer
	 */
	public void forceRefreshConnections() {
		if (networkInitializer != null) {
			networkInitializer.forceRefreshConnections();
			List<WifiAwareModels.ConnectionInfo> connections = 
				networkInitializer.getActiveConnections();
			wifiAwareConnections.postValue(connections);
		}
		// Also trigger a stats update
		updateNetworkStats();
	}

	/**
	 * Force refresh of clients list
	 */
	public void forceRefreshClients() {
		// Clear current clients list and force refresh
		connectedClients.setValue(new ArrayList<>());

	}

	// Getters for LiveData
	public LiveData<String> getText() {
		return mText;
	}

	public LiveData<List<WifiAwareClientInfo>> getConnectedClients() {
		return connectedClients;
	}

	public LiveData<List<WifiAwareModels.ConnectionInfo>> getWifiAwareConnections() {
		return wifiAwareConnections;
	}

	public LiveData<NetworkStats> getNetworkStats() {
		return networkStats;
	}

	public LiveData<Boolean> getIsPlayerReady() {
		return isPlayerReady;
	}

	public LiveData<Boolean> getIsStreamConnected() {
		return isStreamConnected;
	}

	public ExoPlayer getPlayer() {
		return player;
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
			
			// Properly cleanup Rust service connections
			cleanupRustConnections();
			
			releasePlayer();
			currentStreamUri = null;
			isStreamConnected.setValue(false);
			isPlayerReady.setValue(false);
		}
	}

	private void cleanupRustConnections() {
		try {
			// Call native cleanup method to remove all active sockets
			RustService.cleanupAllSockets();
			
			// Clean up NAN data paths if WiFi Aware is active
			AppContainer container = AppContainer.getInstance();
			if (container.getNetworkInitializer() != null) {
				// Force cleanup of inactive data paths
				container.getNetworkInitializer().cleanupInactiveDataPaths();
			}
		} catch (Exception e) {
			// Log error but continue with cleanup
			android.util.Log.e("StreamingViewModel", "Error during Rust service cleanup", e);
		}
	}

	private void releasePlayer() {
		if (player != null) {
			player.release();
			player = null;
		}
		if (pipe != null) {
			try {
				pipe[0].close();
				pipe[1].close();
			} catch (IOException ignored) {}
			pipe = null;
		}
		if (statsTimer != null) {
			statsTimer.cancel();
			statsTimer = null;
		}
	}

	@Override
	protected void onCleared() {
		super.onCleared();
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
				android.util.Log.w("StreamingViewModel", "Failed to extract host from URI: " + uriString, e);
			}
		}
		
		// Final fallback
		return "stream1.iris.dcc.fc.up.pt";
	}

	public String getDeviceCapabilities() {
		if (networkInitializer != null) {
			return networkInitializer.getDeviceCapabilitiesSummary();
		}
		return "N/A";
	}
}
