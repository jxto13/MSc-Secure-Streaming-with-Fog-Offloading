package pt.iris.network;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.wifi.aware.PeerHandle;
import android.os.Build;
import android.util.Log;

import androidx.annotation.RequiresApi;

import java.util.List;
import java.util.UUID;

import pt.iris.network.lib.WifiAwareLibrary;
import pt.iris.network.lib.WifiAwareModels;
import pt.iris.services.RustService;
import pt.iris.utils.AppContainer;

/**
 * NetworkInitializer - Simplified wrapper around the WiFi Aware Library
 * 
 * This class now acts as a simple adapter between the app and the modular WiFi Aware library.
 * All core functionality has been moved to the pt.iris.network.lib package for better modularity.
 */
@SuppressLint("MissingPermission")
public class NetworkInitializer implements WifiAwareLibrary.WifiAwareLibraryCallback {
	private static final String TAG = "NetworkInitializer";
	public static final String SERVICE_NAME = "Iris_Streaming_Service";

	private final Context context;
	private WifiAwareLibrary wifiAwareLibrary;
	private UUID myUUID;

	public NetworkInitializer(Context context) {
		this.context = context;
	}

	/**
	 * Entrypoint for the NetworkInitializer
	 */
	public void start() {
		this.myUUID = AppContainer.getInstance().getMyUUID();
		
		Log.d(TAG, "NetworkInitializer starting with UUID: " + myUUID.toString());
		
		// Initialize the WiFi Aware library
		wifiAwareLibrary = new WifiAwareLibrary(context, SERVICE_NAME, myUUID, this);
		wifiAwareLibrary.initialize();
	}

	// ========== WifiAwareLibrary.WifiAwareLibraryCallback Implementation ==========

	@Override
	public void onWifiAwareReady() {
		Log.d(TAG, "WiFi Aware library is ready and operational");
	}

	@Override
	public void onWifiAwareFailed(String reason) {
		Log.e(TAG, "WiFi Aware library initialization failed: " + reason);
	}

	@Override
	public void onDataPathEstablished(PeerHandle peerHandle, String ipAddress) {
		Log.d(TAG, "Data path established with peer " + peerHandle + " at IPv6: " + ipAddress);
		// Log updated statistics
		logDataPathStats();
	}

	@Override
	public void onDataPathLost(PeerHandle peerHandle) {
		Log.w(TAG, "Data path lost with peer: " + peerHandle);
		// Log updated statistics
		logDataPathStats();
	}

	@Override
	public void onDataPathFailed(PeerHandle peerHandle, String reason) {
		Log.e(TAG, "Data path failed with peer " + peerHandle + ": " + reason);
	}

	@Override
	public void addSocketToRust(String ipAddress) {
		// Add this socket to the Rust layer for streaming communication
		RustService.addSocket(ipAddress);
		Log.d(TAG, "Added socket to Rust layer: " + ipAddress);
	}


	// ========== Public API Methods (delegated to library) ==========

	/**
	 * Get current data path statistics for UI display
	 */
	public String getDataPathStatistics() {
		if (wifiAwareLibrary != null) {
			return wifiAwareLibrary.getDataPathStatistics();
		}
		return "WiFi Aware Library not initialized";
	}


	/**
	 * Get the number of active data paths
	 */
	public int getActiveDataPathCount() {
		return wifiAwareLibrary != null ? wifiAwareLibrary.getActiveDataPathCount() : 0;
	}

	/**
	 * Get active WiFi Aware data path connections for UI display
	 */
	public List<WifiAwareModels.ConnectionInfo> getActiveConnections() {
		if (wifiAwareLibrary != null) {
			return wifiAwareLibrary.getActiveConnections();
		}
		return new java.util.ArrayList<>();
	}

	/**
	 * Force refresh of connections (used by UI refresh buttons)
	 */
	public void forceRefreshConnections() {
		// Log current state for debugging
		Log.d(TAG, "Force refresh: " + getDataPathStatistics());
	}

	/**
	 * Stops Wi-Fi Aware and destroys all resources created by it
	 */
	public void closeSessions() {
		Log.d(TAG, "Closing all WiFi Aware sessions");
		
		if (wifiAwareLibrary != null) {
			wifiAwareLibrary.shutdown();
			wifiAwareLibrary = null;
		}
	}

	/**
	 * Log current data path statistics for monitoring and debugging
	 */
	private void logDataPathStats() {
		if (wifiAwareLibrary != null) {
			Log.d(TAG, "=== Current Statistics ===");
			Log.d(TAG, wifiAwareLibrary.getDataPathStatistics());
		}
	}

}
