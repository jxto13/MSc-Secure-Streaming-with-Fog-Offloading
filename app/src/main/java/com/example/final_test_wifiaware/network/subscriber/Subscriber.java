package com.example.final_test_wifiaware.network.subscriber;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.utils.Connection;
import com.example.final_test_wifiaware.ui.home.HomeViewModel;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.ArrayList;
import java.util.List;

public class Subscriber {
	private final Context context;
	private final WifiAwareSession mWifiAwareSession;
	private final DiscoverySessionCallback subscriberSessionCallback;
	private final ConnectivityManager.NetworkCallback subscriberNetworkCallback;
	private SubscribeDiscoverySession subscriberDiscoverySession;
	private ConnectivityManager connMgr ;
	private List<Connection> connections = new ArrayList<>();
	private HomeViewModel homeViewModel;
	public Subscriber(WifiAwareSession session, Context inputContext) {
		Log.d("Subscriber", "Starting Subscriber");
		this.mWifiAwareSession = session;
		this.context = inputContext;
		this.connMgr = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);

		homeViewModel = AppContainer.getInstance().getHomeViewModel();

		this.subscriberSessionCallback = new SubscriberDiscoverySessionCallback(this);
		this.subscriberNetworkCallback = new SubscriberConnectivityManagerCallback(this);

		// All initialized can start subscriber
		startSubscriber();
	}

	/**
	 * Create a SubscribeConfig for subscribing to data
	 */
	public static SubscribeConfig createSubscribeConfig() {
		return new SubscribeConfig.Builder()
				.setServiceName("test")
				.build();
	}

	@SuppressLint("MissingPermission")
	public void startSubscriber() {
		mWifiAwareSession.subscribe(Subscriber.createSubscribeConfig(), subscriberSessionCallback, null);
	}

	public void initializeSocketConnectionSubscriber(Connection target) {

		// Step 2: Create a WifiAwareNetworkSpecifier
		NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(subscriberDiscoverySession, target.getTarget())
				.setPskPassphrase("somePassword")
				.build();


		// Step 3: Create a NetworkRequest and set up a network callback
		NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
				.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
				.setNetworkSpecifier(networkSpecifier)
				.build();


		// Create a new Manager Callback and add to connection
		target.setConnectivityManagerCallback(new SubscriberConnectivityManagerCallback(this));
		connMgr.requestNetwork(myNetworkRequest,target.getConnectivityManagerCallback());
	}
	public void destroySpecificResource(Connection connection){
		connMgr.unregisterNetworkCallback(connection.getConnectivityManagerCallback());
	}

	public void destroyResources(){
		for (Connection item : connections ) {
			if (item.getConnectedStatus()){
				Log.d("Destroy Resources", "unregisterNetworkCallback - " + item.getUUID().toString());
				connMgr.unregisterNetworkCallback(item.getConnectivityManagerCallback());
			}
		}

	}

	// Getters and Setters
	public void setSubscriberDiscoverySession(SubscribeDiscoverySession subscriberDiscoverySession) {
		this.subscriberDiscoverySession = subscriberDiscoverySession;
	}
	public SubscribeDiscoverySession getSubscriberDiscoverySession() {
		return subscriberDiscoverySession;
	}

	public ConnectivityManager.NetworkCallback getSubscriberNetworkCallback() {
		return subscriberNetworkCallback;
	}
	public List<Connection> getConnections() {
		return connections;
	}

	public HomeViewModel getHomeViewModel() {
		return homeViewModel;
	}


}

