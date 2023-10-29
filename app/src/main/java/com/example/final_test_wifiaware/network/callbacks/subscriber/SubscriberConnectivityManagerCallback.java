package com.example.final_test_wifiaware.network.callbacks.subscriber;

import android.net.Network;
import android.net.NetworkCapabilities;
import android.util.Log;

import com.example.final_test_wifiaware.network.callbacks.AbstractConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.ui.home.HomeFragment;

public class SubscriberConnectivityManagerCallback extends AbstractConnectivityManagerCallback {
	private Subscriber subscriber;
	public SubscriberConnectivityManagerCallback(Subscriber inputSubscriber){
		Log.d("Callback", "SubscriberConnectivityManagerCallback Started");
		this.subscriber = inputSubscriber;
	}

	@Override
	public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
		Log.d("NetworkCallback", "Network capabilities have changed");
		if(networkCapabilities != null){
			Log.d("NetworkCallback", "Network capabilities is" + networkCapabilities.getTransportInfo());
			Log.d("NetworkCallback", "Network UpstreamBandwidthKbps is " + networkCapabilities.getLinkUpstreamBandwidthKbps());
			Log.d("NetworkCallback", "Network DownstreamBandwidthKbps is " + networkCapabilities.getLinkDownstreamBandwidthKbps());
			Log.d("NetworkCallback", "Network SignalStrength is " + networkCapabilities.getSignalStrength());
		}
		subscriber.setNetworkCapabilities(networkCapabilities);
		//networkPeer = network;
	}


}
