package com.example.final_test_wifiaware.network.callbacks.subscriber;

import android.net.Network;
import android.net.NetworkCapabilities;
import android.util.Log;

import androidx.annotation.NonNull;

import com.example.final_test_wifiaware.network.callbacks.AbstractConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;

public class SubscriberConnectivityManagerCallback extends AbstractConnectivityManagerCallback {
	private Subscriber subscriber;
	public SubscriberConnectivityManagerCallback(Subscriber inputSubscriber){
		Log.d("Callback", "SubscriberConnectivityManagerCallback Started");
		this.subscriber = inputSubscriber;
	}

	@Override
	public void onCapabilitiesChanged(@NonNull Network network, @NonNull NetworkCapabilities networkCapabilities) {
		Log.d("NetworkCallback", "Subscriber Network capabilities have changed");
		Log.d("NetworkCallback", "Subscriber Network capabilities is" + networkCapabilities.getTransportInfo());
		Log.d("NetworkCallback", "Subscriber Network UpstreamBandwidthKbps is " + networkCapabilities.getLinkUpstreamBandwidthKbps());
		Log.d("NetworkCallback", "Subscriber Network DownstreamBandwidthKbps is " + networkCapabilities.getLinkDownstreamBandwidthKbps());
		Log.d("NetworkCallback", "Subscriber Network SignalStrength is " + networkCapabilities.getSignalStrength());
		//subscriber.setNetworkCapabilities(networkCapabilities);
		//subscriber.networkPeer = network;
	}


}
