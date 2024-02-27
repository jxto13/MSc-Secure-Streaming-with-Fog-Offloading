package com.example.final_test_wifiaware.network.connection;

import static com.example.final_test_wifiaware.network.protocol.MessageType.FOUND_PEER;
import static com.example.final_test_wifiaware.network.protocol.MessageType.START_CONNECTION;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.wifi.aware.Characteristics;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareDataPathSecurityConfig;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Build;
import android.os.Handler;
import android.util.Log;

import androidx.annotation.NonNull;
import androidx.annotation.RequiresApi;

import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.Arrays;
import java.util.List;
import java.util.UUID;

public class SubscriberClient {
	private final String TAG = "Subscriber Client";
	private final Context context;
	private final WifiAwareSession mWifiAwareSession;
	private final Handler workerHandle;
	private final WifiAwareManager wifiAwareManager;
	private final ConnectivityManager connectivityManager;
	private SubscribeDiscoverySession subscribeSession;

	public SubscriberClient(Context context, WifiAwareSession mWifiAwareSession, Handler workerHandle) {
		this.context = context;
		this.mWifiAwareSession = mWifiAwareSession;
		this.workerHandle = workerHandle;
		this.wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);
		this.connectivityManager =  (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
	}

	@SuppressLint("MissingPermission")
	public void start(String serviceName){

		// Configuring and starting the subscription session
		SubscribeConfig subscribeConfig = new SubscribeConfig.Builder()
				.setServiceName(serviceName)
				.build();

		mWifiAwareSession.subscribe(subscribeConfig, new DiscoverySessionCallback() {

			@Override
			public void onSubscribeStarted(@NonNull SubscribeDiscoverySession session) {
				Log.d(TAG,"Subscriber Started");
				// Subscription session started successfully
				subscribeSession = session;
			}

			@Override
			public void onSessionConfigFailed() {
				// Handling failed session configuration for subscription
				subscribeSession = null;
			}

			@Override
			public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
				subscribeSession.sendMessage(peerHandle, 0, FOUND_PEER);
			}

			@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
			@Override
			public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
				if(Arrays.equals(message, START_CONNECTION)) {

					// Example security configuration
					WifiAwareDataPathSecurityConfig securityConfig = new WifiAwareDataPathSecurityConfig.Builder(Characteristics.WIFI_AWARE_CIPHER_SUITE_NCS_SK_128)
							.setPskPassphrase("!9WDp4EU!#ax8Cv$hk*3Ca")
							.build();

					// Build network specifier for the connection
					WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(subscribeSession, peerHandle)
							//.setDataPathSecurityConfig(securityConfig)
							.build();


					// Build network request for the connection
					NetworkRequest networkRequest = new NetworkRequest.Builder()
							.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
							.addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_RESTRICTED) // Not restricted by system policy
							.addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_METERED) // Not metered
					        .addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_VPN) // Not a VPN
							//.addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_ROAMING) // Not roaming
							//.addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_CONGESTED) // Not congested
							.addCapability(NetworkCapabilities.NET_CAPABILITY_TRUSTED) // Trusted
							.setNetworkSpecifier(networkSpecifier)
							.build();

					ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
						@Override
						public void onAvailable(Network network) {
							// Add log statement for onAvailable
							Log.d(TAG, "Network is now available: " + network.toString());
						}

						@Override
						public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
							// Add log statement for onCapabilitiesChanged
							Log.d(TAG, "Network capabilities changed: " + networkCapabilities.toString());
						}

						@Override
						public void onLost(Network network) {
							// Add log statement for onLost
							Log.d(TAG, "Network is now lost: " + network.toString());
						}

					};
					Log.d("Sub Client",networkRequest.toString());
					// Request a network with the specified network request and register network callback
					connectivityManager.requestNetwork(networkRequest, callback);
				}

			}

			@Override
			public void onMessageSendSucceeded(int messageId) {
				// Handling successful message send
			}
		}, workerHandle);

	}

	public void stop(){
		if(subscribeSession != null){
			subscribeSession.close();
			subscribeSession = null;
		}
	}




}
