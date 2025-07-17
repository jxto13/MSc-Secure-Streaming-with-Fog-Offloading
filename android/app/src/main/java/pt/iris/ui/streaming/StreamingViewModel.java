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
import pt.iris.utils.AppContainer;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Timer;
import java.util.TimerTask;

public class StreamingViewModel extends ViewModel {

	private final MutableLiveData<String> mText;
	private final MutableLiveData<NetworkStats> networkStats;
	private final MutableLiveData<Boolean> isPlayerReady;
	private final MutableLiveData<Boolean> isStreamConnected;
	
	private ExoPlayer player;
	private ParcelFileDescriptor[] pipe;
	private Timer statsTimer;
	private NetworkInitializer networkInitializer;
	private Uri currentStreamUri;

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
			
			// Start native client with the stream URL and check if connection succeeded
			boolean connectionSuccessful = RustService.startClient(writePfd.getFd(), streamUrl);
			
			// If connection was successful, trigger network initializer via AppContainer
			if (connectionSuccessful) {
				android.util.Log.i("StreamingViewModel", "Client connected successfully, triggering WiFi Aware initialization");
				try {
					pt.iris.utils.AppContainer.getInstance().triggerNetworkInitialization();
				} catch (Exception e) {
					android.util.Log.w("StreamingViewModel", "Failed to trigger network initialization: " + e.getMessage());
				}
			} else {
				android.util.Log.e("StreamingViewModel", "Failed to connect to server");
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
			throw new RuntimeException("Failed to initialize player", e);
		}

		return player;
	}

	private void startStatsCollection() {
		if (statsTimer != null) {
			statsTimer.cancel();
		}
		
		statsTimer = new Timer();
		statsTimer.schedule(new TimerTask() {
			@Override
			public void run() {
				updateNetworkStats();
			}
		}, 0, 1000); // Update every 1 second (auto-refresh)
	}

	private void updateNetworkStats() {
		NetworkStats stats = networkStats.getValue();
		if (stats == null) stats = new NetworkStats();
		

		networkStats.postValue(stats);
	}

	// Getters for LiveData
	public LiveData<String> getText() {
		return mText;
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

			releasePlayer();
			currentStreamUri = null;
			isStreamConnected.setValue(false);
			isPlayerReady.setValue(false);
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

}
