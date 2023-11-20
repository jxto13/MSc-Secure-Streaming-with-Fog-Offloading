package com.example.final_test_wifiaware.network.callbacks.subscriber;

import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.util.Log;

import androidx.annotation.NonNull;

import com.example.final_test_wifiaware.network.callbacks.AbstractConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.network.connections.Connection;
import com.example.final_test_wifiaware.utils.AppContainer;

public class SubscriberConnectivityManagerCallback extends AbstractConnectivityManagerCallback {
	private final Subscriber subscriber;
	public SubscriberConnectivityManagerCallback(Subscriber inputSubscriber){
		Log.d("Callback", "SubscriberConnectivityManagerCallback Started");
		this.subscriber = inputSubscriber;
	}

	@Override
	public void onAvailable(@NonNull Network network) {


	}

	@Override
	public void onCapabilitiesChanged(@NonNull Network network, @NonNull NetworkCapabilities networkCapabilities) {

		 Log.d("NetworkCallback", "Subscriber Network capabilities have changed");
		 Log.d("NetworkCallback", "Subscriber Network capabilities is" + networkCapabilities.getTransportInfo());
		 Log.d("NetworkCallback", "Subscriber Network UpstreamBandwidthKbps is " + networkCapabilities.getLinkUpstreamBandwidthKbps());
		 Log.d("NetworkCallback", "Subscriber Network DownstreamBandwidthKbps is " + networkCapabilities.getLinkDownstreamBandwidthKbps());
		 Log.d("NetworkCallback", "Subscriber Network SignalStrength is " + networkCapabilities.getSignalStrength());
		 WifiAwareNetworkInfo wifiAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();

		 String ipv6 = wifiAwareInfo.getPeerIpv6Addr().toString().substring(1);
		 //int indexOfPercentage = ipv6.indexOf('%');
		 //String newString = ipv6.substring(1, indexOfPercentage);

		 //String response = AppContainer.getInstance().getGrpcClient().sayHello(newString);

		 //String response = AppContainer.getInstance().getGrpcClient().sayHello(ipv6);
		 //Log.d("gRPC Response",response);
		 //Log.d("gRPC Response",wifiAwareInfo.getPeerIpv6Addr().toString());
		 //subscriber.setNetworkCapabilities(networkCapabilities);
		 //subscriber.networkPeer = network;

	}
}
