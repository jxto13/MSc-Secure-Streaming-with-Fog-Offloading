package pt.iris.network.utils;

import android.net.MacAddress;
import android.net.wifi.aware.IdentityChangedListener;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import androidx.annotation.NonNull;

public abstract class ClusterIdTracker extends IdentityChangedListener {
	private static final long DEBOUNCE_DELAY_MS = 5000; // Adjust the delay as needed
	private final Handler handler;
	private MacAddress lastStableClusterId;
	private Runnable debounceRunnable;

	public abstract void onStableClusterId(MacAddress stableClusterId);

	public ClusterIdTracker() {
		this.handler = new Handler(Looper.getMainLooper());
	}

	@Override
	public void onIdentityChanged(byte[] mac) {
		// Handle identity changes if needed
		Log.d("onIdentityChanged", convertBytesToMacAddress(mac));
	}

	@Override
	public void onClusterIdChanged(int clusterEventType, @NonNull MacAddress clusterId) {
		final MacAddress currentClusterId = clusterId;
		Log.d("onClusterIdChanged", String.format("%s to clusterId = %s",
				(clusterEventType == CLUSTER_CHANGE_EVENT_STARTED) ? "CLUSTER_CHANGE_EVENT_STARTED" : "CLUSTER_CHANGE_EVENT_JOINED",
				clusterId
		));
		if (isStableChange(currentClusterId)) {
			lastStableClusterId = currentClusterId;
			runDebouncedCode(currentClusterId);
		}
	}

	private boolean isStableChange(MacAddress currentClusterId) {
		return !currentClusterId.equals(lastStableClusterId);
	}

	private void runDebouncedCode(final MacAddress clusterId) {
		if (debounceRunnable != null) {
			handler.removeCallbacks(debounceRunnable);
		}

		debounceRunnable = () -> {
			// Run your code here with the stable clusterId
			Log.d("StableClusterId", "Stable clusterId = " + clusterId);
			onStableClusterId(clusterId);
		};

		handler.postDelayed(debounceRunnable, DEBOUNCE_DELAY_MS);
	}

	private static String convertBytesToMacAddress(byte[] macAddressBytes){
		if (macAddressBytes.length != 6) {
			throw new IllegalArgumentException("MAC address should be 6 bytes long");
		}

		StringBuilder macAddressBuilder = new StringBuilder(18);

		for (byte b : macAddressBytes) {
			// Convert byte to hexadecimal string
			macAddressBuilder.append(String.format("%02X", b));
			// Add ":" separator between bytes
			macAddressBuilder.append(":");
		}

		// Remove the last ":" separator
		macAddressBuilder.setLength(macAddressBuilder.length() - 1);

		return macAddressBuilder.toString();
	}

}

