package com.example.final_test_wifiaware.network;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.util.Log;

import androidx.annotation.Nullable;
import androidx.core.app.ActivityCompat;

import com.example.final_test_wifiaware.network.connections.ConnectionManager;
import com.example.final_test_wifiaware.network.publisher.Publisher;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.io.IOException;
import java.net.ServerSocket;
import java.util.List;
import java.util.PriorityQueue;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public class NetworkInitializer {
	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	private Subscriber subscriber;
	private Publisher publisher;
	private UUID myUUID;
	private ConnectionManager connectionManager;

	public NetworkInitializer(Context context) {
		this.context = context;
	}

	/**
	 * Entrypoint for the NetworkInitializer
	 */
	public void start() {
		this.myUUID = AppContainer.getInstance().getMyUUID();
		// Set UUID to the UI
		AppContainer.getInstance().getHomeViewModel().updateMyUUID(myUUID.toString());

		// Start wifiAwareManager
		startWifiAwareManager();

		// Start WifiAwareSession
		createWifiAwareSession();
	}

	/**
	 * Initialize Wi-Fi Aware Manager
	 */
	private void startWifiAwareManager() {
		wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			wifiAwareManagerStatus = true;
			Log.d("wifiAwareManager", "available - getAvailableDataPathsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount()
					+ " - getAvailableSubscribeSessionsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount()
					+ " - getAvailablePublishSessionsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());
		} else {
			wifiAwareManagerStatus = false;
			Log.d("wifiAwareManager", "not available");
			// Wi-Fi Aware is not available on this device
		}
	}

	/**
	 * Attach to Wi-Fi Aware and create a session
	 */
	private void createWifiAwareSession() {
		if (wifiAwareManagerStatus) {
			wifiAwareManager.attach(new AttachCallback() {
				@Override
				public void onAttached(WifiAwareSession session) {
					mWifiAwareSession = session;
					Log.d("AttachCallback", "Session creation successfully");

					connectionManager = new ConnectionManager(context);
					AppContainer.getInstance().setConnectionManager(connectionManager);

					// Session is created, can start the subscriber and publisher
					subscriber = new Subscriber(mWifiAwareSession, context, myUUID);
					publisher = new Publisher(mWifiAwareSession, context, myUUID);

					// Add to AppContainer
					AppContainer.getInstance().setSubscriber(subscriber);
					AppContainer.getInstance().setPublisher(publisher);


				}

				@Override
				public void onAttachFailed() {
					// Wi-Fi Aware attachment failed
					Log.d("AttachCallback", "Session creation failed");
				}
			}, null);
		}
	}

	/**
	 * Stops Wi-Fi Aware and destroys all resources created by it
	 */
	public void stopWifiAware() {

		if (mWifiAwareSession != null) {
			// Close and destroy all resource in publisher and subscriber
			connectionManager.destroyResources();

			// Close the Wi-Fi Aware session to terminate publish and subscribe sessions
			mWifiAwareSession.close();
		}
	}


	// Getters and Setters

	public UUID getMyUUID() {
		return myUUID;
	}
}
