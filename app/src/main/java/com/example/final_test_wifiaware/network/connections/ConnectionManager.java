package com.example.final_test_wifiaware.network.connections;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.wifi.aware.PeerHandle;
import android.util.Log;

import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.UUID;

public class ConnectionManager {
	Context context;
	private final ConnectivityManager connMgr;
	Set<UUID> uuidSet = new HashSet<>(); // Set to keep track of peerHandle changes of UUID, efficient way instead Log(N)
	private final List<Connection> connections = new ArrayList<>();

	public ConnectionManager(Context context) {
		this.context = context;
		this.connMgr = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
		System.out.println(context);
	}


	// Check if connections exists for that uuid, if true set the peerHandle (can be a new one), if not create a connection with that uuid
	public void checkConnectionExists(UUID uuid, PeerHandle peerHandle){

		if(uuidSet.contains(uuid)){
			Log.d("ServiceDiscovered", "Service name: " + uuid.toString() + " already exists");

			for (Connection connection: connections) {
				if (connection.getUUID().equals(uuid)){
					connection.setPeerHandle(peerHandle);
				}
			}
		}else{
			System.out.println(uuid.toString());
			// Create new connection from uuid service info and peer handle
			connections.add(new Connection(uuid, peerHandle));
			AppContainer.getInstance().getHomeViewModel().updateConnections(connections);

			// Add to UUID Set
			uuidSet.add(uuid);
		}
	}

	// Create network from subscriber
	public void createSubscriberNetwork(UUID uuid, PeerHandle peerHandle){
		Connection tmp = new Connection(uuid, peerHandle);
		tmp.setConnectedStatus(true);
		connections.add(tmp);
		AppContainer.getInstance().getSubscriber().initializeSocketConnectionSubscriber(tmp);

		for (Connection connection: connections) {
			if (connection.getUUID().equals(uuid)){
				//System.out.println(peerHandle.hashCode() + " - " + connection.getPeerHandle().hashCode());
				connection.setPeerHandle(peerHandle);
				//AppContainer.getInstance().getSubscriber().initializeSocketConnectionSubscriber(connection);
			}
		}

	}

	public void destroyResources(){
		for (Connection item : connections ) {
			if (item.getConnectedStatus()){
				Log.d("Destroy Resources", "unregisterNetworkCallback - " + item.getUUID().toString());
				connMgr.unregisterNetworkCallback(item.getSubscriberConnectivityManagerCallback());
				connMgr.unregisterNetworkCallback(item.getPublisherConnectivityManagerCallback());

			}
		}
	}

}
