package com.example.final_test_wifiaware.network.publisher;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.publisher.PublisherConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.publisher.PublisherDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.ui.home.CustomAdapter;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.net.DatagramSocket;
import java.net.SocketException;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

public class Publisher {
	private final Context context;
	private final WifiAwareSession mWifiAwareSession;
	private final DiscoverySessionCallback publisherSessionCallback;
	private ConnectivityManager.NetworkCallback publisherNetworkCallback;
	private PublishDiscoverySession publishDiscoverySession;
	private ConnectivityManager connMgr;
	private static UUID myUUID;
	public Publisher(WifiAwareSession session, Context inputContext) {
		Log.d("Publisher", "Starting Publisher");
		this.mWifiAwareSession = session;
		this.context = inputContext;
		this.connMgr = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);

		this.publisherSessionCallback = new PublisherDiscoverySessionCallback(this);
		this.publisherNetworkCallback = new PublisherConnectivityManagerCallback(this);

		// Generate my UUID
		myUUID = UUID.randomUUID();

		// Set UUID to the UI
		AppContainer.getInstance().getHomeViewModel().updateMyUUID(myUUID.toString());

		// All initialized can start publisher
		startPublisher();
	}

	/**
	 * Create a SubscribeConfig for subscribing to data
	 */
	public static PublishConfig createPublishConfig() {
		return new PublishConfig.Builder()
				.setServiceSpecificInfo(Utils.UUIDtoByteArray(myUUID)) // Generate random uuid and convert to bytes
				.setServiceName("test") // Set the same service name as in publish
				//.setRangingEnabled(true)
				.build();
	}

	@SuppressLint("MissingPermission")
	public void startPublisher() {
		mWifiAwareSession.publish(Publisher.createPublishConfig(), publisherSessionCallback, null);
	}

	public void initializeSocketConnectionPublisher() {

			//Create a WifiAwareNetworkSpecifier
			NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(publishDiscoverySession)
					.setPskPassphrase("somePassword")
					.setTransportProtocol(17) // set udp transport protocol
					.setPort(5050)
					.build();

			//Create a NetworkRequest and set up a network callback
			NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
					.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
					.setNetworkSpecifier(networkSpecifier)
					.build();


			connMgr.requestNetwork(myNetworkRequest, publisherNetworkCallback);
	}

	public void destroyResources(){
		Log.d("Destroy Resources", "unregisterNetworkCallback - publisherNetworkCallback ");
		connMgr.unregisterNetworkCallback(publisherNetworkCallback);
	}

	// Getters and Setters
	public void setPublishDiscoverySession(PublishDiscoverySession publishDiscoverySession) {
		this.publishDiscoverySession = publishDiscoverySession;
	}

	public PublishDiscoverySession getPublishDiscoverySession() {
		return publishDiscoverySession;
	}

	public ConnectivityManager.NetworkCallback getPublisherNetworkCallback() {
		return publisherNetworkCallback;
	}

	public static UUID getMyUUID() {
		return myUUID;
	}
}
