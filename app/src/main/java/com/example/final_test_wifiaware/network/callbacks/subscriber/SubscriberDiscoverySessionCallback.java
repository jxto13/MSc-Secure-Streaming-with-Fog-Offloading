package com.example.final_test_wifiaware.network.callbacks.subscriber;

import android.content.Context;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.AbstractDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.ui.home.HomeFragment;

import java.util.List;

public class SubscriberDiscoverySessionCallback extends AbstractDiscoverySessionCallback {
	private final Context context;
	private Subscriber subscriber;
	public SubscriberDiscoverySessionCallback(Context inputContext, Subscriber inputSubscriber){
		Log.d("Callback", "DiscoverySessionCallback Started");
		this.context = inputContext;
		this.subscriber = inputSubscriber;
	}
	@Override
	public void onServiceDiscoveredWithinRange(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter, int distanceMm) {
		Log.d("ServiceDiscovered", "Service name: " + new String(serviceSpecificInfo) + " | peerHandle: " + peerHandle.hashCode() + " range: "
		+ distanceMm);

	}
	@Override
	public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
		// Handle discovered services here

		Log.d("ServiceDiscovered", "Service name: " + new String(serviceSpecificInfo) + " | peerHandle: " + peerHandle.hashCode());

		// Subscribe to the service here using the known service name or any criteria you require.
		//subscriber.subscribeDiscoverySession.sendMessage(peerHandle,1,"subscriverID".getBytes());

	}

	@Override
	public void onSubscribeStarted(SubscribeDiscoverySession session) {
		// Subscribe session started, you can now receive data
		Log.d("onSubscribeStarted", session.toString());
		subscriber.setSubscribeDiscoverySession(session);
	}



	@Override
	public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
		Log.d("Message","received from " + peerHandle.hashCode() + ": " + new String(message));
		String messageString = new String(message);
		switch (messageString){
			case "subscriverID":
				break;
			case "publisherID":
				subscriber.setPublisherPeerHandle(peerHandle);
				//Subscriber.initializeSocketConnectionSubscriber();
				break;
		}


	}

}
