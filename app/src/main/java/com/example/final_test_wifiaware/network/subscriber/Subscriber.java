package com.example.final_test_wifiaware.network.subscriber;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberDiscoverySessionCallback;
import com.example.final_test_wifiaware.ui.home.HomeFragment;

import java.net.Inet6Address;

public class Subscriber {
	private Context context;
	private WifiAwareSession mWifiAwareSession;
	private DiscoverySessionCallback subscriberSessionCallback;
	private ConnectivityManager.NetworkCallback subscriberNetworkCallback;

	private NetworkCapabilities networkCapabilities;

	private SubscribeDiscoverySession subscribeDiscoverySession;



	private PeerHandle publisherPeerHandle;
	public Subscriber(WifiAwareSession session, Context inputContext) {
		Log.d("Subscriber", "Starting Subscriber");
		this.mWifiAwareSession = session;
		this.context = inputContext;

		this.subscriberSessionCallback = new SubscriberDiscoverySessionCallback(context,this);
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

	public void initializeSocketConnectionSubscriber() {

		// Step 2: Create a WifiAwareNetworkSpecifier
		NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(subscribeDiscoverySession, publisherPeerHandle)
				.setPskPassphrase("somePassword")
				.build();


		// Step 3: Create a NetworkRequest and set up a network callback
		NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
				.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
				.setNetworkSpecifier(networkSpecifier)
				.build();


		ConnectivityManager connMgr = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
		connMgr.requestNetwork(myNetworkRequest, subscriberNetworkCallback);


	}

	public void createSocket() {
		WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();
		assert peerAwareInfo != null;
		Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
		int peerPort = peerAwareInfo.getPort();
		Log.d("createSocket", "peerIpv6:" + peerIpv6 + " | " + networkCapabilities.toString());


	}

	// Getters and Setters
	public void setSubscribeDiscoverySession(SubscribeDiscoverySession subscribeDiscoverySession) {
		this.subscribeDiscoverySession = subscribeDiscoverySession;
	}

	public void setNetworkCapabilities(NetworkCapabilities networkCapabilities) {
		this.networkCapabilities = networkCapabilities;
	}
	public void setPublisherPeerHandle(PeerHandle publisherPeerHandle) {
		this.publisherPeerHandle = publisherPeerHandle;
	}
}

