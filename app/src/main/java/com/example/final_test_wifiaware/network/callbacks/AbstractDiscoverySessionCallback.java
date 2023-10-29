package com.example.final_test_wifiaware.network.callbacks;

import android.net.wifi.aware.DiscoverySessionCallback;
import android.util.Log;

public abstract class AbstractDiscoverySessionCallback  extends DiscoverySessionCallback {

	@Override
	public void onMessageSendSucceeded(int messageId) {
		Log.d("Message","send successfully");
	}
	@Override
	public void onMessageSendFailed(int messageId) {
		Log.d("Message","send failed");

	}
	@Override
	public void onSessionConfigFailed() {
		Log.e("SessionConfigFailed", "Session configuration failed");
		// Session configuration failed
	}
}
