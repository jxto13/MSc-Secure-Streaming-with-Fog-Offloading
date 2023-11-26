package com.example.final_test_wifiaware.network.connections;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.wifi.aware.PeerHandle;
import android.util.Log;

import com.example.final_test_wifiaware.network.protocol.TCPEchoServer;
import com.example.final_test_wifiaware.utils.AppContainer;
import com.example.final_test_wifiaware.network.protocol.TCPEchoClient;

import java.io.IOException;
import java.net.Inet6Address;
import java.net.ServerSocket;
import java.net.Socket;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

public class ConnectionManager {
	Context context;
	private final ConnectivityManager connMgr;
	private final List<Connection> connections = new ArrayList<>();

	public ConnectionManager(Context context) {
		this.context = context;
		this.connMgr = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
	}

	// Check if connections exists for that uuid, if true set the peerHandle (can be a new one), if not create a connection with that uuid
	public void checkConnectionExists(UUID uuid, PeerHandle peerHandle){

		int result = uuid.compareTo(AppContainer.getInstance().getMyUUID());

		if (result < 0) {
			System.out.println(uuid + " is smaller than " + AppContainer.getInstance().getMyUUID());

			// Create new connection from uuid service info and peer handle
			Connection tmp =  new Connection(uuid, peerHandle);
			connections.add(tmp);
			AppContainer.getInstance().getHomeViewModel().updateConnections(connections);

			AppContainer.getInstance().getPublisher().initializeSocketConnectionPublisher(tmp);
		}

	}

	public void createSubscriberNetwork(UUID uuid, PeerHandle peerHandle){
		Connection tmp = new Connection(uuid, peerHandle);
		connections.add(tmp);
		AppContainer.getInstance().getHomeViewModel().updateConnections(connections);

		AppContainer.getInstance().getSubscriber().initializeSocketConnectionSubscriber(tmp);

	}

	public void startHeartHandlerClient(Network network, Inet6Address peerIpv6, int peerPort){
		TCPEchoClient echoClient = new TCPEchoClient(network, peerIpv6, peerPort);
		echoClient.start();
	}

	public void destroyResources(){
		for (Connection item : connections ) {
			Log.d("Destroy Resources", "unregisterNetworkCallback - " + item.getUUID().toString());
			if(item.getSubscriberConnectivityManagerCallback() != null){
				connMgr.unregisterNetworkCallback(item.getSubscriberConnectivityManagerCallback());
			}
			if(item.getPublisherConnectivityManagerCallback() != null) {
				connMgr.unregisterNetworkCallback(item.getPublisherConnectivityManagerCallback());
			}
		}
	}

	// Create network from subscriber

	public List<Connection> getConnections() {
		return connections;
	}
}
