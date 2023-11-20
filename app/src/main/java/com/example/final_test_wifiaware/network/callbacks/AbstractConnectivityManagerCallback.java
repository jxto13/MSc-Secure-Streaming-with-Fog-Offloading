package com.example.final_test_wifiaware.network.callbacks;

import android.net.ConnectivityManager;
import android.net.Network;
import android.util.Log;

import androidx.annotation.NonNull;

public abstract class AbstractConnectivityManagerCallback extends ConnectivityManager.NetworkCallback {

	/*@Override
	public void onAvailable(@NonNull Network network) {
		Log.d("NetworkCallback", "Network is available " + network.hashCode());
	}*/

	@Override
	public void onLost(@NonNull Network network) {
		Log.d("NetworkCallback", "Network is lost");
	}
}
