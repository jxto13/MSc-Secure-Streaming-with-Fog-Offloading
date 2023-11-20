package com.example.final_test_wifiaware.network.callbacks.publisher;

import android.annotation.SuppressLint;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishDiscoverySession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.AbstractDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.publisher.Publisher;

public class PublisherDiscoverySessionCallback extends AbstractDiscoverySessionCallback {

	private final Publisher publisher;

	public PublisherDiscoverySessionCallback(Publisher inputPublisher){
		Log.d("Callback", "DiscoverySessionCallback Started");
		this.publisher = inputPublisher;
	}

	@Override
	public void onPublishStarted(PublishDiscoverySession session) {
		// Publish session started
		Log.d("onPublishStarted", session.toString());
		publisher.setPublishDiscoverySession(session);
	}



}
