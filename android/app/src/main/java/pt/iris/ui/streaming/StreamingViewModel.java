package pt.iris.ui.streaming;

import android.content.Context;
import android.net.Uri;
import android.net.wifi.aware.PeerHandle;
import android.os.Build;
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
import java.util.Objects;
import java.util.Timer;
import java.util.TimerTask;
import java.util.concurrent.ConcurrentHashMap;

public class StreamingViewModel extends ViewModel implements NetworkInitializer.NetworkConnectionListener {

	private final MutableLiveData<String> mText;
	private final MutableLiveData<NetworkStats> networkStats;
	private final MutableLiveData<Boolean> isPlayerReady;
	private final MutableLiveData<Boolean> isStreamConnected;
	private final MutableLiveData<Map<PeerHandle, DataPathInfo>> wifiAwareConnections;
	private final MutableLiveData<SessionInfo> wifiAwareSessionInfo;
	private final MutableLiveData<List<String>> connectionLogs;
	private Map<PeerHandle, WifiAwareModels.DeviceRole> peerRoles = new ConcurrentHashMap<>();

	private ExoPlayer player;
	private ParcelFileDescriptor[] pipe;
	private Timer statsTimer;
	private NetworkInitializer networkInitializer;
	private Uri currentStreamUri;

	// Bitrate sampling state
	private long lastBytesSample = -1L;
	private long lastSampleTimeMs = -1L;
	private double emaBitratebps = 0.0;
	private static final double BITRATE_EMA_ALPHA = 0.3;

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

		try {
			networkInitializer = AppContainer.getInstance().getNetworkInitializer();
			if (networkInitializer != null) {
				networkInitializer.addListener(this);
			}
		} catch (Exception e) {
			networkInitializer = null;
		}

		startStatsCollection();
	}

	@OptIn(markerClass = UnstableApi.class)
	public ExoPlayer initializePlayer(Context context, Uri streamUri) {
		if (player != null) {
			releasePlayer();
		}

		try {
			pipe = ParcelFileDescriptor.createPipe();
			ParcelFileDescriptor readPfd = pipe[0];
			ParcelFileDescriptor writePfd = pipe[1];

			String streamUrl = extractStreamUrlFromUri(streamUri);

			boolean connectionSuccessful = RustService.startClient(writePfd.getFd(), streamUrl);

			if (connectionSuccessful) {
				Log.i(TAG, "Client connected successfully, triggering WiFi Aware initialization");
				isStreamConnected.postValue(true);
				try {
					AppContainer.getInstance().triggerNetworkInitialization();
				} catch (Exception e) {
					Log.w(TAG, "Failed to trigger network initialization: " + e.getMessage());
				}
			} else {
				Log.e(TAG, "Failed to connect to server");
				isStreamConnected.postValue(false);
				throw new RuntimeException("Failed to connect to server");
			}

			player = new ExoPlayer.Builder(context)
					.setMediaSourceFactory(new DefaultMediaSourceFactory(new PfdPipeDataSource.Factory(readPfd)))
					.build();

			MediaItem mediaItem = MediaItem.fromUri(streamUri);
			player.setMediaItem(mediaItem);
			player.prepare();

			player.addListener(new androidx.media3.common.Player.Listener() {
				@Override
				public void onPlaybackStateChanged(int playbackState) {
					isPlayerReady.setValue(playbackState == androidx.media3.common.Player.STATE_READY);
				}
			});

			startStatsCollection();

		} catch (IOException e) {
			isStreamConnected.postValue(false);
			throw new RuntimeException("Failed to initialize player", e);
		}

		return player;
	}

	private void startStatsCollection() {
		if (statsTimer != null) {
			statsTimer.cancel();
			statsTimer = null;
		}

		statsTimer = new Timer("StreamingViewModel-StatsTimer", true);
		statsTimer.schedule(new TimerTask() {
			@Override
			public void run() {
				updateNetworkStats();
			}
		}, 0, 1000);
	}

	private void updateNetworkStats() {
		NetworkStats stats = networkStats.getValue();
		if (stats == null) {
			stats = new NetworkStats();
		}

		if (networkInitializer != null) {
			if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
				try {
					int available = Objects.requireNonNull(
							networkInitializer.getWifiAwareLibrary().wifiAwareManager.getAvailableAwareResources()
					).getAvailableDataPathsCount();
					int supported = networkInitializer.getWifiAwareLibrary().wifiAwareManager
							.getCharacteristics().getNumberOfSupportedDataPaths();
					stats.wifiAwareDataPaths = available + "/" + supported;
				} catch (Exception ignored) {}
			}
			updateSessionInfo();
		}

		stats.packetLossRate = RustService.getPacketLossPercentage();

		// Instantaneous bitrate using deltas + EMA smoothing
		try {
			PfdPipeDataSource pfdSource = PfdPipeDataSource.Factory.getLastInstance();
			if (pfdSource != null) {
				long totalBytes = pfdSource.getTotalBytesRead();
				stats.totalBytesReceived = totalBytes;

				long nowMs = android.os.SystemClock.elapsedRealtime();

				if (lastBytesSample >= 0 && lastSampleTimeMs >= 0) {
					long bytesDelta = totalBytes - lastBytesSample;
					long timeDeltaMs = Math.max(1, nowMs - lastSampleTimeMs);
					double instBitratebps = (bytesDelta * 8.0) / (timeDeltaMs / 1000.0);

					if (emaBitratebps == 0.0) {
						emaBitratebps = instBitratebps;
					} else {
						emaBitratebps = BITRATE_EMA_ALPHA * instBitratebps
								+ (1.0 - BITRATE_EMA_ALPHA) * emaBitratebps;
					}
					stats.currentBitrate = emaBitratebps;
				}

				lastBytesSample = totalBytes;
				lastSampleTimeMs = nowMs;
			}
		} catch (Exception ignored) {}

		stats.uptime += 1;

		networkStats.postValue(stats);
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
			wifiAwareConnections.postValue(new ConcurrentHashMap<>());
		}
	}

	@Override
	public void onError(String error) {
		Log.e(TAG, "Wi-Fi Aware error: " + error);
	}

	public void connectToPeer(PeerHandle peerHandle) {
		if (networkInitializer != null) {
			networkInitializer.getWifiAwareLibrary().establishDataPath(peerHandle, true);
			Log.d(TAG, "Requested connection to peer: " + peerHandle);
		} else {
			Log.w(TAG, "NetworkInitializer is null, cannot connect to peer");
		}
	}

	public Map<PeerHandle, WifiAwareModels.DeviceRole> getPeerRoles() {
		if (networkInitializer != null && networkInitializer.getWifiAwareLibrary() != null) {
			try {
				java.lang.reflect.Field field = networkInitializer.getWifiAwareLibrary().getClass().getDeclaredField("peerRoles");
				field.setAccessible(true);
				Object value = field.get(networkInitializer.getWifiAwareLibrary());
				if (value instanceof Map) {
					//noinspection unchecked
					peerRoles = new ConcurrentHashMap<>((Map<PeerHandle, WifiAwareModels.DeviceRole>) value);
				}
			} catch (Exception ignored) {}
		}
		return peerRoles;
	}

	@Override
	public void onRoleChanged(WifiAwareModels.DeviceRole newRole) {
		Log.d(TAG, "Role changed to: " + newRole);
		updateSessionInfo();
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
		player = initializePlayer(context, streamUri);
		currentStreamUri = streamUri;
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
		if (statsTimer != null) {
			statsTimer.cancel();
			statsTimer = null;
		}

		try {
			RustService.stopClient();
		} catch (Exception e) {
			Log.w(TAG, "Error stopping Rust client", e);
		}

		if (player != null) {
			player.release();
			player = null;
		}

		// reset bitrate sampler state
		lastBytesSample = -1L;
		lastSampleTimeMs = -1L;
		emaBitratebps = 0.0;
	}

	@Override
	protected void onCleared() {
		super.onCleared();
		if (networkInitializer != null) {
			networkInitializer.removeListener(this);
		}
		releasePlayer();
	}

	private String extractStreamUrlFromUri(Uri uri) {
		if (uri == null) {
			return "stream1.iris.dcc.fc.up.pt";
		}

		String host = uri.getHost();
		if (host != null && !host.isEmpty()) {
			return host;
		}

		String uriString = uri.toString();
		if (uriString.startsWith("iris://")) {
			try {
				String withoutProtocol = uriString.substring(7);
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
				Log.w(TAG, "Failed to extract host from URI: " + uriString, e);
			}
		}

		return "stream1.iris.dcc.fc.up.pt";
	}

	private void updateSessionInfo() {
		if (networkInitializer != null && networkInitializer.getWifiAwareLibrary() != null) {
			SessionInfo sessionInfo = networkInitializer.getWifiAwareLibrary().getSessionInfo();
			wifiAwareSessionInfo.postValue(sessionInfo);
		}
	}

	public void stopStreaming() {
		releasePlayer();
	}
}
