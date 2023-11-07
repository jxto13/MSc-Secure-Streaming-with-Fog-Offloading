package com.example.final_test_wifiaware.network.callbacks.publisher;

import android.net.Network;
import android.net.NetworkCapabilities;
import android.util.Log;

import androidx.annotation.NonNull;

import com.example.final_test_wifiaware.network.callbacks.AbstractConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.publisher.Publisher;
import com.example.final_test_wifiaware.ui.home.HomeFragment;

public class PublisherConnectivityManagerCallback extends AbstractConnectivityManagerCallback {
	private Publisher publisher;
	public PublisherConnectivityManagerCallback(Publisher inputPublisher){
		Log.d("Callback", "PublisherConnectivityManagerCallback Started");
		this.publisher = inputPublisher;
	}

	@Override
	public void onCapabilitiesChanged(@NonNull Network network, @NonNull NetworkCapabilities networkCapabilities) {
		Log.d("NetworkCallback", "Publisher Network capabilities have changed");
		Log.d("NetworkCallback", "Publisher Network capabilities is" + networkCapabilities.getTransportInfo());
		Log.d("NetworkCallback", "Publisher Network UpstreamBandwidthKbps is " + networkCapabilities.getLinkUpstreamBandwidthKbps());
		Log.d("NetworkCallback", "Publisher Network DownstreamBandwidthKbps is " + networkCapabilities.getLinkDownstreamBandwidthKbps());
		Log.d("NetworkCallback", "Publisher Network SignalStrength is " + networkCapabilities.getSignalStrength());

	}
	@Override
	public void onAvailable(Network network){
		//send message to peer-handler
		//Log.d("Message", "Send message to subscriber to start network");
	}


}
