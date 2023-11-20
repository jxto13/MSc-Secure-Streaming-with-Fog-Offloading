package com.example.final_test_wifiaware.network.callbacks;

import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.util.Log;

import androidx.annotation.NonNull;

import com.example.final_test_wifiaware.network.protocol.MessageHandler;

public abstract class AbstractDiscoverySessionCallback  extends DiscoverySessionCallback {
	MessageHandler messageHandler = new MessageHandler();
	@Override
	public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
		messageHandler.parseMessage(peerHandle, message);
	}

	@Override
	public void onMessageSendSucceeded(int messageId) {
		Log.d("Message","messageId: " + messageId + " successfully");
	}

	@Override
	public void onMessageSendFailed(int messageId) {
		Log.d("Message","messageId: " + messageId + " failed");
	}

	@Override
	public void onServiceLost(@NonNull PeerHandle peerHandle, int reason) {
		Log.d("ServiceLost", String.valueOf(peerHandle.hashCode()));
	}
}
