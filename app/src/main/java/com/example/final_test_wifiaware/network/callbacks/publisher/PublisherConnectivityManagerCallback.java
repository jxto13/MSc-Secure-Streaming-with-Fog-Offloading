package com.example.final_test_wifiaware.network.callbacks.publisher;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.util.Log;

import androidx.annotation.NonNull;

import com.example.final_test_wifiaware.network.connections.Connection;
import com.example.final_test_wifiaware.network.connections.ConnectionManager;
import com.example.final_test_wifiaware.network.publisher.Publisher;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.net.Inet6Address;
import java.util.ArrayList;
import java.util.List;

public class PublisherConnectivityManagerCallback extends ConnectivityManager.NetworkCallback {
	private Publisher publisher;
	private ConnectionManager connectionManager = AppContainer.getInstance().getConnectionManager();

	public PublisherConnectivityManagerCallback(Publisher inputPublisher){
		Log.d("Callback", "PublisherConnectivityManagerCallback Started");
		this.publisher = inputPublisher;
	}

	@Override
	public void onAvailable(Network network){

		WifiAwareManager wifiAwareManager = (WifiAwareManager) publisher.getContext().getSystemService(Context.WIFI_AWARE_SERVICE);
		Log.d("wifiAwareManager", "publisher network " + network + " available - getAvailableDataPathsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount()
				+ " - getAvailableSubscribeSessionsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount()
				+ " - getAvailablePublishSessionsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());

		ConnectivityManager connectivity = (ConnectivityManager) publisher.getContext().getSystemService(Context.CONNECTIVITY_SERVICE);
		NetworkCapabilities networkCapabilities = connectivity.getNetworkCapabilities(network);


		Log.d("networkCapabilities", networkCapabilities.toString());

	}


	@Override
	public void onLost(@NonNull Network network) {
		Log.d("NetworkCallback", "Network is lost");
	}
}
