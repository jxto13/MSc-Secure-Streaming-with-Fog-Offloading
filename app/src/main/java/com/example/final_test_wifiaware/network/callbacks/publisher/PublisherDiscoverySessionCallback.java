package com.example.final_test_wifiaware.network.callbacks.publisher;

import android.content.Context;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.AbstractDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.publisher.Publisher;
import com.example.final_test_wifiaware.ui.home.HomeFragment;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;

import java.util.List;

public class PublisherDiscoverySessionCallback extends AbstractDiscoverySessionCallback {
	private final Context context;
	private final Publisher publisher;

	public PublisherDiscoverySessionCallback(Context inputContext, Publisher inputPublisher){
		Log.d("Callback", "DiscoverySessionCallback Started");
		this.context = inputContext;
		this.publisher = inputPublisher;
	}

	@Override
	public void onPublishStarted(PublishDiscoverySession session) {
		// Publish session started, you can now send data
		Log.d("onPublishStarted", session.toString());
		publisher.setPublishDiscoverySession(session);
	}


	@Override
	public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
		Log.d("Message","received from " + peerHandle.hashCode() + ": " + new String(message));
		String messageString = new String(message);
		switch (messageString){
			case "subscriverID":
				break;
			case "publisherID":
				//HomeFragment.publisherPeerHandle = peerHandle;
				//Subscriber.initializeSocketConnectionSubscriber();
				break;
		}


	}

}
