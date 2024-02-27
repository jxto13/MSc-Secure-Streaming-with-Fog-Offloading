package com.example.final_test_wifiaware.network.utils;

import android.os.AsyncTask;
import android.util.Log;
import java.io.IOException;
import java.net.InetAddress;
import java.net.UnknownHostException;

public class PingLinkLocalDevicesTask extends AsyncTask<Void, Void, Void> {

	private static final String TAG = "PingLinkLocalDevicesTask";
	private volatile boolean isScanning = true; // Flag to control scanning

	@Override
	protected Void doInBackground(Void... voids) {
		// Define the IPv6 link-local prefix
		String linkLocalPrefix = "fe80::";

		// Iterate over possible interface identifiers (last 64 bits)
		for (int i = 1; i < Math.pow(2, 64) - 1 && isScanning; i++) {
			// Construct the link-local address
			String linkLocalAddress = linkLocalPrefix + Integer.toHexString(i);

			try {
				// Ping the address
				InetAddress inetAddress = InetAddress.getByName(linkLocalAddress);
				boolean reachable = inetAddress.isReachable(1000); // Timeout in milliseconds

				// Check if the address is reachable
				if (reachable) {
					Log.d(TAG, "Link-local device found at address: " + linkLocalAddress);
				} else {
					Log.d(TAG, "No link-local device found at address: " + linkLocalAddress);
				}
			} catch (UnknownHostException e) {
				Log.e(TAG, "Unknown host: " + linkLocalAddress);
			} catch (IOException e) {
				Log.e(TAG, "IO Exception occurred while pinging: " + linkLocalAddress);
			}
		}

		return null;
	}

	// Method to stop scanning
	public void stopScanning() {
		isScanning = false;
	}
}

