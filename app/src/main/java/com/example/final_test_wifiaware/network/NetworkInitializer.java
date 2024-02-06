package com.example.final_test_wifiaware.network;

import static java.lang.Integer.parseInt;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.MacAddress;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.Log;

import androidx.annotation.NonNull;
import androidx.annotation.RequiresApi;

import com.example.final_test_wifiaware.network.protocol.Client;
import com.example.final_test_wifiaware.network.protocol.Server;

import com.example.final_test_wifiaware.network.utils.ClusterIdTracker;
import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;

import android.net.wifi.aware.Characteristics;

@SuppressLint("MissingPermission")
public class NetworkInitializer {
	private static final String TAG = "NetworkInitializer";

	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	private UUID myUUID;
	private ConnectivityManager connectivityManager;
	private WifiAwareManager manager;
	private WifiAwareSession session;
	private PublishDiscoverySession publishSession;
	private SubscribeDiscoverySession subscribeSession;
	private Server publishServer;
	private List<Client> subscriberClientList;
	private HandlerThread worker;
	private Handler workerHandle;
	public NetworkInitializer(Context context) {
		this.context = context;
	}

	/**
	 * Entrypoint for the NetworkInitializer
	 */
	public void start() {
		this.myUUID = AppContainer.getInstance().getMyUUID();

		// Set UUID to the UI
		AppContainer.getInstance().getHomeViewModel().updateMyUUID(myUUID.toString());

		this.connectivityManager =  (ConnectivityManager) context.getSystemService(context.CONNECTIVITY_SERVICE);

		// Start wifiAwareManager
		startWifiAwareManager();

		// Create and start a worker thread for background tasks
		worker = new HandlerThread("Worker");
		worker.start();
		workerHandle = new Handler(worker.getLooper());

		// Initialize the list to store subscriber client instances
		subscriberClientList = new ArrayList<>();

		// Start WifiAwareSession
		createWifiAwareSession();
	}

	/**
	 * Initialize Wi-Fi Aware Manager
	 */
	@SuppressLint({"DefaultLocale", "NewApi"})
	private void startWifiAwareManager() {
		wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			wifiAwareManagerStatus = true;

			Characteristics characteristics = wifiAwareManager.getCharacteristics();

			if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {

				assert characteristics != null;
				Log.d(TAG,String.format(
						"isAwarePairingSupported() - %b \n" +
						"isInstantCommunicationModeSupported() - %b \n" +
						"isSetChannelOnDataPathSupported() - %b \n" +
						"getNumberOfSupportedDataInterfaces - %d \n" +
						"getNumberOfSupportedDataPaths - %d \n",
						characteristics.isAwarePairingSupported(),
						characteristics.isInstantCommunicationModeSupported(),
						wifiAwareManager.isSetChannelOnDataPathSupported(),
						characteristics.getNumberOfSupportedDataInterfaces(),
						characteristics.getNumberOfSupportedDataPaths()
				));
			}
			Log.d(TAG, String.format("getAvailableDataPathsCount() = %d \n" +
							"getAvailableSubscribeSessionsCount() = %d \n" +
							"getAvailablePublishSessionsCount() = %d\n",
					wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount(),
					wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount(),
					wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount()));
		} else {
			wifiAwareManagerStatus = false;
			Log.d("wifiAwareManager", "not available");
			// Wi-Fi Aware is not available on this device
		}
	}

	/**
	 * Attach to Wi-Fi Aware and create a session
	 */
	private void createWifiAwareSession() {
		if (wifiAwareManagerStatus) {

			final byte[] START_SEQUENCE = { (byte) 0x01, (byte) 0x01};
			final byte[] START_CONNECTION = { (byte) 0x01, (byte) 0x02 };
			final byte[] FOUND_PEER = { (byte) 0x01, (byte) 0x03};

			wifiAwareManager.attach(new AttachCallback() {
				@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
				@Override
				public void onAttached(WifiAwareSession session) {
					// Wi-Fi Aware session successfully attached
					mWifiAwareSession = session;
					Log.d("AttachCallback", "Session creation successfully");

				}

				@Override
				public void onAttachFailed() {
					// Wi-Fi Aware attachment failed
					Log.d("AttachCallback", "Session creation failed");
				}

				@Override
				public void onAwareSessionTerminated(){
					Log.d("onAwareSessionTerminated", "Session closed, needs to reopen new session");
				}
			},new ClusterIdTracker(){

				@Override
				public void onStableClusterId(MacAddress stableClusterId) {
					// Your custom logic with stableClusterId
					Log.d("CustomLogic", "Stable clusterId: " + stableClusterId);

					// Configuring and starting the publication session
					PublishConfig publishConfig = new PublishConfig.Builder()
							.setServiceName("serviceName")
							.setServiceSpecificInfo(Utils.asBytes(myUUID))
							.build();

					mWifiAwareSession.publish(publishConfig, new DiscoverySessionCallback() {

						@Override
						public void onPublishStarted(@NonNull PublishDiscoverySession session) {
							// Publication session started successfully
							publishSession = session;
							AppContainer.getInstance().getHomeViewModel().updatePublisherSessionID("seasonId: " + session.hashCode());
						}

						@Override
						public void onSessionConfigFailed() {
							// Handling failed session configuration
						}

						@Override
						public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
							if(Arrays.equals(message, FOUND_PEER)){

								// Build network specifier for the connection
								WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(publishSession, peerHandle)
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
										Log.d("PublishNetworkCallback", "Network is now available: " + network.toString());
									}

									@Override
									public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
										// Add log statement for onCapabilitiesChanged
										Log.d("PublishNetworkCallback", "Network capabilities changed: " + networkCapabilities.toString());
									}

									@Override
									public void onLost(Network network) {
										// Add log statement for onLost
										Log.d("PublishNetworkCallback", "Network is now lost: " + network.toString());
									}

								};

								connectivityManager.requestNetwork(networkRequest, callback);
								publishSession.sendMessage(peerHandle,0,START_CONNECTION);
							}


						}
					}, workerHandle);

					// Configuring and starting the subscription session
					SubscribeConfig subscribeConfig = new SubscribeConfig.Builder()
							.setServiceName("serviceName")
							.build();

					mWifiAwareSession.subscribe(subscribeConfig, new DiscoverySessionCallback() {


						@Override
						public void onSubscribeStarted(@NonNull SubscribeDiscoverySession session) {
							// Subscription session started successfully
							subscribeSession = session;

							AppContainer.getInstance().getHomeViewModel().updateSubscriberSessionID("seasonId: " + session.hashCode());

						}

						@Override
						public void onSessionConfigFailed() {
							// Handling failed session configuration for subscription
							subscribeSession = null;
						}

						@Override
						public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {

							// Check if the discovered service's UUID is less than the local UUID
							if (Utils.asUuid(serviceSpecificInfo).compareTo(myUUID) < 0) {
								subscribeSession.sendMessage(peerHandle,0,FOUND_PEER);
							}

						}

						@Override
						public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
							// Handling received messages for subscription and creating client instances
							//Log.d("onMessageReceived", String.format("Subscribe: Received Message from: %s", Utils.asUuid(message)));

							if(Arrays.equals(message, START_CONNECTION)) {
								// Build network specifier for the connection
								WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(subscribeSession, peerHandle)
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
										Log.d("SubscribeNetworkCallback", "Network is now available: " + network.toString());
									}

									@Override
									public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
										// Add log statement for onCapabilitiesChanged
										Log.d("SubscribeNetworkCallback", "Network capabilities changed: " + networkCapabilities.toString());
									}

									@Override
									public void onLost(Network network) {
										// Add log statement for onLost
										Log.d("SubscribeNetworkCallback", "Network is now lost: " + network.toString());
									}

								};

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


			}, workerHandle);
		}
	}





	/**
	 * Stops Wi-Fi Aware and destroys all resources created by it
	 */

	public void closeSessions(){
		if(publishSession != null){
			publishSession.close();
			publishSession = null;
		}
		if(subscribeSession != null){
			subscribeSession.close();
			subscribeSession = null;
		}
		if(mWifiAwareSession != null){
			mWifiAwareSession.close();
			mWifiAwareSession = null;
		}
	}




	// Getters and Setters

}
