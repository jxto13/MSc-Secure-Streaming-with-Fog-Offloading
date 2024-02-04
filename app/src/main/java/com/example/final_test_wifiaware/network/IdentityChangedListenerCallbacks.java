package com.example.final_test_wifiaware.network;

import android.net.MacAddress;
import android.net.wifi.aware.IdentityChangedListener;
import android.util.Log;

import androidx.annotation.NonNull;

import java.net.SocketException;
import java.util.Arrays;

public class IdentityChangedListenerCallbacks extends IdentityChangedListener {

	public IdentityChangedListenerCallbacks() {
		super();
	}

	@Override
	public void onIdentityChanged(byte[] mac) {
		Log.d("onIdentityChanged", convertBytesToMacAddress(mac));
		super.onIdentityChanged(mac);
	}

	@Override
	public void onClusterIdChanged(int clusterEventType, @NonNull MacAddress clusterId) {

		Log.d("onClusterIdChanged", String.format("%s to clusterId = %s",
				(clusterEventType == CLUSTER_CHANGE_EVENT_STARTED) ? "CLUSTER_CHANGE_EVENT_STARTED" : "CLUSTER_CHANGE_EVENT_JOINED",
				clusterId.toOuiString()
		));
		super.onClusterIdChanged(clusterEventType, clusterId);
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
