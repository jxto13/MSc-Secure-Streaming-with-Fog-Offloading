package com.example.final_test_wifiaware.network.callbacks.publisher;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.util.Log;

import androidx.annotation.NonNull;

import com.example.final_test_wifiaware.network.callbacks.AbstractDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.publisher.Publisher;
import com.example.final_test_wifiaware.ui.home.HomeFragment;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;

import java.util.ArrayList;
import java.util.List;

public class PublisherDiscoverySessionCallback extends AbstractDiscoverySessionCallback {

	private final Publisher publisher;

	public PublisherDiscoverySessionCallback(Publisher inputPublisher){
		Log.d("Callback", "DiscoverySessionCallback Started");
		this.publisher = inputPublisher;
	}

	@Override
	public void onPublishStarted(PublishDiscoverySession session) {
		// Publish session started, you can now send data
		Log.d("onPublishStarted", session.toString());
		publisher.setPublishDiscoverySession(session);
		publisher.initializeSocketConnectionPublisher();

	}

	@Override
	public void onServiceLost(@NonNull PeerHandle peerHandle, int reason) {
		Log.d("ServiceLost", String.valueOf(peerHandle.hashCode()));
	}

	@SuppressLint("NotifyDataSetChanged")
	@Override
	public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
		Log.d("Message","received from " + peerHandle.hashCode() + ": " + new String(message));
		String messageString = new String(message);
		switch (messageString){
			case "subscriverID":
				//publisher.getPublishDiscoverySession().sendMessage(peerHandle, 2, "networkCreation".getBytes());
				break;
		}
	}

}
