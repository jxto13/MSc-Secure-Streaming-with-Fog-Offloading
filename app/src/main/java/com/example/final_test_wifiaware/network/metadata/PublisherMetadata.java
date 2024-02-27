package com.example.final_test_wifiaware.network.metadata;

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
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
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

import com.example.final_test_wifiaware.network.protocol.MessageType;
import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.Arrays;
import java.util.List;

public class PublisherMetadata {
	private final String TAG = "Publishe Metadata";
	private final String serviceName = "metadata";
	private final Context context;
	private final WifiAwareSession mWifiAwareSession;
	private final Handler workerHandle;
	private final WifiAwareManager wifiAwareManager;
	private final ConnectivityManager connectivityManager;
	private PublishDiscoverySession publishSession;

	public PublisherMetadata(Context context, WifiAwareSession mWifiAwareSession, Handler workerHandle) {
		this.context = context;
		this.mWifiAwareSession = mWifiAwareSession;
		this.workerHandle = workerHandle;
		this.wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);
		this.connectivityManager =  (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
	}

	@SuppressLint("MissingPermission")
	public void start(){

		// Configuring and starting the publication session
		PublishConfig publishConfig = new PublishConfig.Builder()
				.setServiceName(serviceName)
				.setServiceSpecificInfo(Utils.asBytes(AppContainer.getInstance().getMyUUID()))
				.build();

		mWifiAwareSession.publish(publishConfig, new DiscoverySessionCallback() {

			@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
			@Override
			public void onPublishStarted(@NonNull PublishDiscoverySession session) {
				Log.d(TAG,"Publisher Started");
				// Publication session started successfully
				publishSession = session;
			}

			@Override
			public void onSessionConfigFailed() {
				// Handling failed session configuration
			}

			@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
			@Override
			public void onMessageReceived(PeerHandle peerHandle, byte[] message) {


				// Example security configuration
				WifiAwareDataPathSecurityConfig securityConfig = new WifiAwareDataPathSecurityConfig.Builder(Characteristics.WIFI_AWARE_CIPHER_SUITE_NCS_SK_128)
						.setPskPassphrase("!9WDp4EU!#ax8Cv$hk*3Ca")
						.build();

				// Build network specifier for the connection
				WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(publishSession, peerHandle)
						//.setDataPathSecurityConfig(securityConfig)
						.build();

				// Build network request for the connection
				NetworkRequest networkRequest = new NetworkRequest.Builder()
						.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
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
				Log.d("Pub Server",networkRequest.toString());
				connectivityManager.requestNetwork(networkRequest, callback);


				// Post a delayed Runnable to send the message after the specified delay
				workerHandle.postDelayed(new Runnable() {
					@Override
					public void run() {
						publishSession.sendMessage(peerHandle, 0, START_CONNECTION);
					}
				}, 2000); // Replace DELAY_MILLIS with the desired delay in milliseconds

			}
		}, workerHandle);

	}

	public void stop(){
		if(publishSession != null){
			publishSession.close();
			publishSession = null;
		}
	}

}
