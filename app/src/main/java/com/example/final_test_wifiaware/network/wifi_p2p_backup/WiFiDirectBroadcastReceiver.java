package com.example.final_test_wifiaware.network.wifi_p2p_backup;

import android.annotation.SuppressLint;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.net.NetworkInfo;
import android.net.wifi.p2p.WifiP2pDevice;
import android.net.wifi.p2p.WifiP2pManager;
import android.util.Log;

import java.util.HashSet;
import java.util.Set;

/**
 * A BroadcastReceiver that notifies of important Wi-Fi p2p events.
 */
public class WiFiDirectBroadcastReceiver extends BroadcastReceiver {

	private static final String TAG = "WiFiDirectReceiver";
	private WifiP2pManager manager;
	private WifiP2pManager.Channel channel;

	// Declare a Set to store unique device addresses
	Set<WifiP2pDevice> discoveredPeers = new HashSet<>();

	private PCLNewsAgency observable;
	public WiFiDirectBroadcastReceiver(WifiP2pManager manager, WifiP2pManager.Channel channel, PCLNewsAgency observable) {
		super();
		this.manager = manager;
		this.channel = channel;
		this.observable = observable;
	}

	@SuppressLint("MissingPermission")
	@Override
	public void onReceive(Context context, Intent intent) {
		String action = intent.getAction();

		if (WifiP2pManager.WIFI_P2P_STATE_CHANGED_ACTION.equals(action)) {
			int state = intent.getIntExtra(WifiP2pManager.EXTRA_WIFI_STATE, -1);
			Log.d(TAG, "Wi-Fi P2P state changed - " + state);
			// Check to see if Wi-Fi is enabled and notify the appropriate activity
		} else if (WifiP2pManager.WIFI_P2P_PEERS_CHANGED_ACTION.equals(action)) {
			Log.d(TAG, "Wi-Fi P2P peers changed");
			// Call WifiP2pManager.requestPeers() to get a list of current peers


			// request available peers from the wifi p2p manager. This is an
			// asynchronous call and the calling activity is notified with a
			// callback on PeerListListener.onPeersAvailable()
			if (manager != null) {
				manager.requestPeers(channel, peerList -> {
					Log.d(TAG, "Available peers requested successfully");

					// Process the list of available peers
					if (peerList != null) {
						for (WifiP2pDevice device : peerList.getDeviceList()) {
							if(!discoveredPeers.contains(device)){
								Log.d(TAG, "Available Peer: " + device.deviceName + " (" + device.deviceAddress + ")");
								discoveredPeers.add(device);
								observable.setNews(device);

							}
						}
					} else {
						Log.e(TAG, "Received null peer list");
					}
				});
			}

		} else if (WifiP2pManager.WIFI_P2P_CONNECTION_CHANGED_ACTION.equals(action)) {
			Log.d(TAG, "Wi-Fi P2P connection changed");
			NetworkInfo networkInfo = intent.getParcelableExtra(WifiP2pManager.EXTRA_NETWORK_INFO);
			if (networkInfo.isConnected()) {
				Log.d(TAG, "Connected to a peer");
				// Respond to a new connection
			} else {
				Log.d(TAG, "Disconnected from a peer");
				// Respond to disconnections
			}
		} else if (WifiP2pManager.WIFI_P2P_THIS_DEVICE_CHANGED_ACTION.equals(action)) {
			Log.d(TAG, "This device's Wi-Fi state changed");
			// Respond to this device's Wi-Fi state changing
			WifiP2pDevice device = intent.getParcelableExtra(WifiP2pManager.EXTRA_WIFI_P2P_DEVICE);
			Log.d(TAG, "Device status - " + device.status);
		}
	}
}
