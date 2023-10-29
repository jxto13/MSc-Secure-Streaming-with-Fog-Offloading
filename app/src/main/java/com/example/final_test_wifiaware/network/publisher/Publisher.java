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

import java.net.DatagramSocket;
import java.net.SocketException;

public class Publisher {
	private Context context;
	private final WifiAwareSession mWifiAwareSession;
	private final DiscoverySessionCallback publisherSessionCallback;
	private ConnectivityManager.NetworkCallback publisherNetworkCallback;

	private PublishDiscoverySession publishDiscoverySession;

	public Publisher(WifiAwareSession session, Context inputContext) {
		Log.d("Publisher", "Starting Publisher");
		this.mWifiAwareSession = session;
		this.context = inputContext;

		this.publisherSessionCallback = new PublisherDiscoverySessionCallback(context,this);
		this.publisherNetworkCallback = new PublisherConnectivityManagerCallback();

		// All initialized can start publisher
		startPublisher();
	}

	/**
	 * Create a SubscribeConfig for subscribing to data
	 */
	public static PublishConfig createPublishConfig() {
		return new PublishConfig.Builder()
				.setServiceSpecificInfo("Stream Video".getBytes())
				.setServiceName("test") // Set the same service name as in publish
				//.setRangingEnabled(true)
				.build();
	}

	@SuppressLint("MissingPermission")
	public void startPublisher() {
		mWifiAwareSession.publish(Publisher.createPublishConfig(), publisherSessionCallback, null);
	}
/*
	public void initializeSocketConnectionPublisher() {
		int counter = 0;
		for (PeerHandle client: connectedClients) {

			// Step 1: Start a ServerSocket on the publisher
			DatagramSocket ss = null;
			try {
				ss = new DatagramSocket(0);
			} catch (SocketException e) {
				throw new RuntimeException(e);
			}


			int port = ss.getLocalPort();


			// Step 2: Create a WifiAwareNetworkSpecifier
			NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(publishDiscoverySession, client)
					.setPskPassphrase("somePassword")
					.setPort(port)
					.build();


			// Step 3: Create a NetworkRequest and set up a network callback
			NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
					.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
					.setNetworkSpecifier(networkSpecifier)
					.build();



			ConnectivityManager connMgr = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
			connMgr.requestNetwork(myNetworkRequest, publisherNetworkCallback);

			//send message to peer-handler
			Log.d("Message", "Send message to subscriber to start network");
			publishDiscoverySession.sendMessage(client, counter++, "publisherID".getBytes());

		}
	}*/

	// Getters and Setters
	public void setPublishDiscoverySession(PublishDiscoverySession publishDiscoverySession) {
		this.publishDiscoverySession = publishDiscoverySession;
	}

}
