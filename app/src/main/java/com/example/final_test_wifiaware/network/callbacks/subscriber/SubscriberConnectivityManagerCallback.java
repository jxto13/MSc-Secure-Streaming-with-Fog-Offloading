package com.example.final_test_wifiaware.network.callbacks.subscriber;

import android.content.Context;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.util.Log;

import androidx.annotation.NonNull;

import com.example.final_test_wifiaware.network.connections.ConnectionManager;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.net.Inet6Address;
import java.net.InetAddress;
import java.net.NetworkInterface;
import java.util.Enumeration;

public class SubscriberConnectivityManagerCallback  extends ConnectivityManager.NetworkCallback {
	private final Subscriber subscriber;
	private ConnectionManager connectionManager = AppContainer.getInstance().getConnectionManager();

	public SubscriberConnectivityManagerCallback(Subscriber inputSubscriber){
		Log.d("Callback", "SubscriberConnectivityManagerCallback Started");
		this.subscriber = inputSubscriber;
	}

	@Override
	public void onAvailable(@NonNull Network network) {

		WifiAwareManager wifiAwareManager = (WifiAwareManager) subscriber.getContext().getSystemService(Context.WIFI_AWARE_SERVICE);
		Log.d("wifiAwareManager", "subscriber network " + network + " available - getAvailableDataPathsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount()
				+ " - getAvailableSubscribeSessionsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount()
				+ " - getAvailablePublishSessionsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());

		ConnectivityManager connectivity = (ConnectivityManager) subscriber.getContext().getSystemService(Context.CONNECTIVITY_SERVICE);
		NetworkCapabilities networkCapabilities = connectivity.getNetworkCapabilities(network);

		WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();
		Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
		int peerPort = peerAwareInfo.getPort();

		Log.d("networkCapabilities", networkCapabilities.toString());
		connectionManager.startHeartHandlerClient(network,peerIpv6,peerPort);


		try {
			Enumeration<NetworkInterface> networkInterfaces = NetworkInterface.getNetworkInterfaces();

			while (networkInterfaces.hasMoreElements()) {
				NetworkInterface networkInterface = networkInterfaces.nextElement();
				Enumeration<InetAddress> inetAddresses = networkInterface.getInetAddresses();

				while (inetAddresses.hasMoreElements()) {
					InetAddress address = inetAddresses.nextElement();

					if (address instanceof Inet6Address) {
						// It's an IPv6 address
						System.out.println("Interface: " + networkInterface.getName());
						System.out.println("IPv6 Address: " + address.getHostAddress());
						System.out.println("---------------------");
					}
				}
			}
		} catch (Exception e) {
			e.printStackTrace();
		}

		String ipv6 = peerAwareInfo.getPeerIpv6Addr().toString().substring(1);
		int indexOfPercentage = ipv6.indexOf('%');
		String newString = ipv6.substring(1, indexOfPercentage);
		Log.d("PeerIPv6", newString);

		//String response = AppContainer.getInstance().getGrpcClient().sayHello(newString);
		//Log.d("gRPC Response",response);
		//Log.d("gRPC Response",wifiAwareInfo.getPeerIpv6Addr().toString());
		//subscriber.setNetworkCapabilities(networkCapabilities);
		//subscriber.networkPeer = network;
	}

	//String ipv6 = wifiAwareInfo.getPeerIpv6Addr().toString().substring(1);

	//int indexOfPercentage = ipv6.indexOf('%');
	//String newString = ipv6.substring(1, indexOfPercentage);

	//String response = AppContainer.getInstance().getGrpcClient().sayHello(newString);

	//String response = AppContainer.getInstance().getGrpcClient().sayHello(ipv6);
	//Log.d("gRPC Response",response);
	//Log.d("gRPC Response",wifiAwareInfo.getPeerIpv6Addr().toString());
	//subscriber.setNetworkCapabilities(networkCapabilities);
	//subscriber.networkPeer = network;

	@Override
	public void onLost(@NonNull Network network) {
		Log.d("NetworkCallback", "Network is lost");
	}
}
