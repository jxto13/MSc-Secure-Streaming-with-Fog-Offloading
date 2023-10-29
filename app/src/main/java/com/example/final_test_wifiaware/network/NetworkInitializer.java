package com.example.final_test_wifiaware.network;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareSession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.publisher.PublisherConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.publisher.PublisherDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.publisher.Publisher;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;

import java.util.ArrayList;
import java.util.List;

public class NetworkInitializer {
	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	public List<PeerHandle> connectedClients = new ArrayList<>();

	private Subscriber subscriber;
	private Publisher publisher;

	public NetworkInitializer(Context context){
		this.context = context;

	}

	public void start(){
		// Start wifiAwareManager
		startWifiAwareManager();

		// Start WifiAwareSession
		createWifiAwareSession();
	}

	/**
	 * Initialize Wi-Fi Aware Manager
	 */
	public void startWifiAwareManager() {
		wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			wifiAwareManagerStatus = true;
			Log.d("wifiAwareManager", "available");
		} else {
			wifiAwareManagerStatus = false;
			Log.d("wifiAwareManager", "not available");
			// Wi-Fi Aware is not available on this device
		}
	}
	/**
	 * Stops Wi-Fi Aware
	 */
	public void stopWifiAware() {
		if (mWifiAwareSession != null) {
			// Close the Wi-Fi Aware session to terminate publish and subscribe sessions
			mWifiAwareSession.close();
		}
	}

	/**
	 * Attach to Wi-Fi Aware and create a session
	 */
	public void createWifiAwareSession() {
		if (wifiAwareManagerStatus) {
			wifiAwareManager.attach(new AttachCallback() {
				@Override
				public void onAttached(WifiAwareSession session) {
					mWifiAwareSession = session;
					Log.d("AttachCallback", "Session creation successfully");

					// Session is created, can start the subscriber and publisher
					subscriber = new Subscriber(mWifiAwareSession, context);
					publisher = new Publisher(mWifiAwareSession, context);
				}

				@Override
				public void onAttachFailed() {
					// Wi-Fi Aware attachment failed
					Log.d("AttachCallback", "Session creation failed");
				}
			}, null);
		}
	}


	// Getters and Setters


}
