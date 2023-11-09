package com.example.final_test_wifiaware.network.callbacks.subscriber;

import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.AbstractDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.network.utils.Connection;
import com.example.final_test_wifiaware.network.utils.Utils;

import java.util.List;
import java.util.UUID;

public class SubscriberDiscoverySessionCallback extends AbstractDiscoverySessionCallback {

	private Subscriber subscriber;
	public SubscriberDiscoverySessionCallback(Subscriber inputSubscriber){
		Log.d("Callback", "DiscoverySessionCallback Started");
		this.subscriber = inputSubscriber;
	}
	@Override
	public void onServiceDiscoveredWithinRange(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter, int distanceMm) {
		Log.d("ServiceDiscovered", "Service name: " + new String(serviceSpecificInfo) + " | peerHandle: " + peerHandle.hashCode() + " range: "
		+ distanceMm);

	}
	@Override
	public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
		UUID uuid = Utils.ByteArraytoUUID(serviceSpecificInfo);
		Log.d("ServiceDiscovered", "Service name: " + uuid.toString() + " | peerHandle: " + peerHandle.hashCode());

		// Create new connection from uuid service info and peer handle
		subscriber.getConnections().add(new Connection(uuid, peerHandle));
		subscriber.getHomeViewModel().updateConnections(subscriber.getConnections());
	}

	@Override
	public void onSubscribeStarted(SubscribeDiscoverySession session) {
		// Subscribe session started, you can now receive data
		Log.d("onSubscribeStarted", session.toString());
		subscriber.setSubscriberDiscoverySession(session);
	}

	@Override
	public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
		Log.d("Message","received from " + peerHandle.hashCode() + ": " + new String(message));
		String messageString = new String(message);
		switch (messageString){
			case "subscriverID":
				break;
			case "networkCreation":
				//subscriber.initializeSocketConnectionSubscriber();
				break;
		}


	}

}
