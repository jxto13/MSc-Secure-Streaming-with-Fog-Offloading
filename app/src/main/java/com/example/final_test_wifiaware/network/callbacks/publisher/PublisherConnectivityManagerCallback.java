package com.example.final_test_wifiaware.network.callbacks.publisher;

import android.net.Network;
import android.net.NetworkCapabilities;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.AbstractConnectivityManagerCallback;
import com.example.final_test_wifiaware.ui.home.HomeFragment;

public class PublisherConnectivityManagerCallback extends AbstractConnectivityManagerCallback {
	public PublisherConnectivityManagerCallback(){
		Log.d("Callback", "PublisherConnectivityManagerCallback Started");
	}

	@Override
	public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
		Log.d("NetworkCallback", "Network capabilities have changed");

	}


}
