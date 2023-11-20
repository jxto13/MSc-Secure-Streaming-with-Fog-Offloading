package com.example.final_test_wifiaware.network.callbacks.subscriber;

import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.AbstractDiscoverySessionCallback;
import com.example.final_test_wifiaware.network.connections.ConnectionManager;
import com.example.final_test_wifiaware.network.protocol.MessageHandler;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.network.connections.Connection;
import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.List;
import java.util.UUID;

public class SubscriberDiscoverySessionCallback extends AbstractDiscoverySessionCallback {

	private final Subscriber subscriber;
	public SubscriberDiscoverySessionCallback(Subscriber inputSubscriber){
		Log.d("Callback", "DiscoverySessionCallback Started");
		this.subscriber = inputSubscriber;
	}
	@Override
	public void onSubscribeStarted(SubscribeDiscoverySession session) {
		// Subscribe session started, you can now receive data
		Log.d("onSubscribeStarted", session.toString());
		subscriber.setSubscriberDiscoverySession(session);
	}

	@Override
	public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
		subscriber.getSubscriberDiscoverySession().sendMessage(peerHandle,1,Utils.StringToBytes("subscriberID:" + subscriber.getMyUUID()));
	}



}
