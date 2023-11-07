package com.example.final_test_wifiaware.network;

import android.app.Activity;
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

import androidx.lifecycle.ViewModelProvider;
import androidx.lifecycle.ViewModelStoreOwner;

import com.example.final_test_wifiaware.network.callbacks.publisher.PublisherConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.publisher.PublisherDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.publisher.Publisher;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.ui.home.CustomAdapter;
import com.example.final_test_wifiaware.ui.home.HomeViewModel;

import java.util.ArrayList;
import java.util.List;

public class NetworkInitializer {
	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	public Subscriber subscriber;
	public Publisher publisher;

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
			Log.d("wifiAwareManager", "available - getAvailableDataPathsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount()
			+ " - getAvailableSubscribeSessionsCount() = "+ wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount()
			+ " - getAvailablePublishSessionsCount() = "+ wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount() );
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

		// Close and destroy all resource in publisher and subscriber
		publisher.destroyResources();
		subscriber.destroyResources();
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
