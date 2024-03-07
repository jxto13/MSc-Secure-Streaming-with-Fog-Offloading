package com.example.final_test_wifiaware.network;


import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.MacAddress;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.Characteristics;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.Log;

import androidx.annotation.RequiresApi;

import com.example.final_test_wifiaware.network.mesh.VirtualNodeWifiAware;
import com.example.final_test_wifiaware.network.utils.ClusterIdTracker;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.UUID;

@SuppressLint("MissingPermission")
public class NetworkInitializer {
	private static final String TAG = "NetworkInitializer";

	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	private UUID myUUID;
	private ConnectivityManager connectivityManager;
	private HandlerThread worker;
	private Handler workerHandle;

	private int MAX_INTERFACES = 0;

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

		// Get instance of ConnectivityManager
		this.connectivityManager =  (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);

		// Create and start a worker thread for background tasks
		worker = new HandlerThread("Worker");
		worker.start();
		workerHandle = new Handler(worker.getLooper());

		// Start wifiAwareManager
		startWifiAwareManager();

		// Start WifiAwareSession
		createWifiAwareSession();
	}

	/**
	 * Initialize Wi-Fi Aware Manager
	 */
	@SuppressLint({"DefaultLocale", "NewApi"})
	private void startWifiAwareManager() {
		wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			wifiAwareManagerStatus = true;

			Characteristics characteristics = wifiAwareManager.getCharacteristics();

			Log.d(TAG, "Aware Pairing Supported: " + characteristics.isAwarePairingSupported());
			Log.d(TAG, "Instant Communication Mode Supported: " + characteristics.isInstantCommunicationModeSupported());
			Log.d(TAG, "Set Channel On Data Path Supported: " + wifiAwareManager.isSetChannelOnDataPathSupported());
			Log.d(TAG, "Number of Supported Data Interfaces: " + characteristics.getNumberOfSupportedDataInterfaces());
			Log.d(TAG, "Number of Supported Data Paths: " + characteristics.getNumberOfSupportedDataPaths());

			Log.d(TAG, "Available Data Paths Count: " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
			Log.d(TAG, "Available Subscribe Sessions Count: " + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount());
			Log.d(TAG, "Available Publish Sessions Count: " + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());

			MAX_INTERFACES = Objects.requireNonNull(wifiAwareManager.getCharacteristics()).getNumberOfSupportedDataInterfaces();

		} else {
			// Wi-Fi Aware is not available on this device
			wifiAwareManagerStatus = false;
			Log.d("wifiAwareManager", "not available");
		}
	}

	/**
	 * Attach to Wi-Fi Aware and create a session
	 */
	private void createWifiAwareSession() {
		if (wifiAwareManagerStatus) {
			wifiAwareManager.attach(new AttachCallback() {
				@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
				@Override
				public void onAttached(WifiAwareSession session) {
					super.onAttached(session);

					// Wi-Fi Aware session successfully attached
					mWifiAwareSession = session;
					Log.d("AttachCallback", "Session creation successfully");
				}

				@Override
				public void onAttachFailed() {
					super.onAttachFailed();

					// Wi-Fi Aware attachment failed
					Log.d("AttachCallback", "Session creation failed");
				}

				@Override
				public void onAwareSessionTerminated(){
					super.onAwareSessionTerminated();
					Log.d("onAwareSessionTerminated", "Session closed, needs to reopen new session");
				}
			},new ClusterIdTracker(){

				@RequiresApi(api = Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
				@Override
				public void onStableClusterId(MacAddress stableClusterId) {
					// Your custom logic with stableClusterId
					Log.d("onStableClusterId", "Stable clusterId: " + stableClusterId);
					AppContainer.getInstance().getHomeViewModel().updateClusterID("" + stableClusterId);

					VirtualNodeWifiAware node = new VirtualNodeWifiAware(mWifiAwareSession, context, myUUID, MAX_INTERFACES);

					switch (node.getNodeType()) {
						case CLIENT -> {
							System.out.println("This is a Client Node");
							node.startClient();
						}
						case RELAY -> {
							System.out.println("This is a Relay Node");
							node.startRelay();
							node.startClient();
						}
						default -> System.out.println("Unknown Node Type");
					}
				}

			}, workerHandle);
		}
	}

	/**
	 * Stops Wi-Fi Aware and destroys all resources created by it
	 */
	public void closeSessions(){
		if(mWifiAwareSession != null){
			mWifiAwareSession.close();
			mWifiAwareSession = null;
		}
	}
	// Getters and Setters

}
