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
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.connections.Connection;
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
	private UUID myUUID;

	public Publisher(WifiAwareSession session, Context inputContext,  UUID myUUID) {
		Log.d("Publisher", "Starting Publisher");
		this.mWifiAwareSession = session;
		this.context = inputContext;
		this.connMgr = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
		this.myUUID = myUUID;

		this.publisherSessionCallback = new PublisherDiscoverySessionCallback(this);
		this.publisherNetworkCallback = new PublisherConnectivityManagerCallback(this);

		// All initialized can start publisher
		startPublisher();
	}

	/**
	 * Create a SubscribeConfig for subscribing to data
	 */
	public static PublishConfig createPublishConfig() {
		return new PublishConfig.Builder()
				.setServiceSpecificInfo(Utils.UUIDtoByteArray(AppContainer.getInstance().getMyUUID())) // Generate random uuid and convert to bytes
				.setServiceName("test") // Set the same service name as in publish
				//.setRangingEnabled(true)
				.build();
	}

	@SuppressLint("MissingPermission")
	public void startPublisher() {
		mWifiAwareSession.publish(Publisher.createPublishConfig(), publisherSessionCallback, null);
	}
	@SuppressLint("NewApi")
	public void initializeSocketConnectionPublisher(Connection target) {

			//Create a WifiAwareNetworkSpecifier

			NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(publishDiscoverySession, target.getPeerHandle())
					.setPskPassphrase("somePassword")
					.build();
		//.setChannelFrequencyMhz(5220, true)
		//.setPskPassphrase("somePassword")

			//Create a NetworkRequest and set up a network callback
			NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
					.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
					.setNetworkSpecifier(networkSpecifier)
					.build();


			// Create a new Manager Callback and add to connection
			target.setPublisherConnectivityManagerCallback(new PublisherConnectivityManagerCallback(this));
			connMgr.requestNetwork(myNetworkRequest,target.getPublisherConnectivityManagerCallback());

			publishDiscoverySession.sendMessage(target.getPeerHandle(), 2,("networkCreation:"+myUUID.toString()).getBytes());
			target.setConnectedStatus(true);
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
	public UUID getMyUUID() {
		return myUUID;
	}

}
