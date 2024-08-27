package pt.iris.network;


import static android.net.wifi.aware.Characteristics.WIFI_AWARE_CIPHER_SUITE_NCS_SK_256;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.MacAddress;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.Characteristics;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.IdentityChangedListener;
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
import android.widget.CompoundButton;

import androidx.annotation.NonNull;
import androidx.annotation.RequiresApi;

import pt.iris.utils.AppContainer;

import java.util.List;
import java.util.Objects;
import java.util.UUID;

@SuppressLint("MissingPermission")
public class NetworkInitializer {
	private static final String TAG = "NetworkInitializer";

	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	private UUID myUUID;
	private ConnectivityManager connectivityManager;
	private HandlerThread worker;
	private Handler workerHandle;

	private int MAX_INTERFACES = 0;
	private int MAX_DATA_PATHS = 0;

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

		// Get instance of ConnectivityManager
		this.connectivityManager =  (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);

		// Create and start a worker thread for background tasks
		worker = new HandlerThread("Worker");
		worker.start();
		workerHandle = new Handler(worker.getLooper());

		// Start wifiAwareManager
		startWifiAwareManager();

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

			Log.d(TAG, "Aware Pairing Supported: " + characteristics.isAwarePairingSupported());
			Log.d(TAG, "Instant Communication Mode Supported: " + characteristics.isInstantCommunicationModeSupported());
			Log.d(TAG, "Set Channel On Data Path Supported: " + wifiAwareManager.isSetChannelOnDataPathSupported());
			Log.d(TAG, "Number of Supported Data Interfaces: " + characteristics.getNumberOfSupportedDataInterfaces());
			Log.d(TAG, "Number of Supported Data Paths: " + characteristics.getNumberOfSupportedDataPaths());

			Log.d(TAG, "Available Data Paths Count: " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
			Log.d(TAG, "Available Subscribe Sessions Count: " + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount());
			Log.d(TAG, "Available Publish Sessions Count: " + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());

			MAX_INTERFACES = Objects.requireNonNull(wifiAwareManager.getCharacteristics()).getNumberOfSupportedDataInterfaces();
			MAX_INTERFACES = Objects.requireNonNull(wifiAwareManager.getCharacteristics()).getNumberOfSupportedDataPaths();

			System.out.println(wifiAwareManager.getCharacteristics().getSupportedCipherSuites());
		} else {
			// Wi-Fi Aware is not available on this device
			wifiAwareManagerStatus = false;
			Log.d("wifiAwareManager", "not available");
		}
	}

	/**
	 * Attach to Wi-Fi Aware and create a session
	 */
	private void createWifiAwareSession() {
		AppContainer appContainer = AppContainer.getInstance();

		if (wifiAwareManagerStatus) {
			wifiAwareManager.attach(new AttachCallback() {

				@Override
				public void onAttached(WifiAwareSession session) {
					super.onAttached(session);
					// Wi-Fi Aware session successfully attached
					Log.d("AttachCallback", "Session creation successfully");

					// Publisher button
					appContainer.getHomeViewModel().getSwitchPublisher().setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
						// Publisher session
						DiscoverySession publisherSession = null;

						@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
						@Override
						public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {
							// Use a switch statement to handle the on and off states
							if (isChecked) {

								// The switch is turned on
								PublishConfig config = new PublishConfig.Builder()
										.setServiceName("Aware_File_Share_Service_Name")
										.build();

								session.publish(config, new DiscoverySessionCallback() {

									@Override
									public void onPublishStarted(PublishDiscoverySession session) {
										Log.d("Publisher", "Publisher Started");
										publisherSession = session;
										appContainer.getHomeViewModel().getPublisherSessionID().setValue(String.valueOf(session.hashCode()));
									}

									@Override
									public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
										NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(publisherSession, peerHandle)
												.setPskPassphrase("somePassword")
												.setPort(8080)
												.build();

										NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
												.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
												.setNetworkSpecifier(networkSpecifier)
												.build();

										ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
											@Override
											public void onAvailable(Network network) {
												Log.d("Publisher Network Manager", "Network available " + network.hashCode());
											}


											@Override
											public void onLost(Network network) {
												Log.d("Publisher Network Manager", "Network lost " + network.hashCode());
											}
										};

										connectivityManager.requestNetwork(myNetworkRequest, callback);

										publisherSession.sendMessage(peerHandle, 0, new byte[0]);

									}
								}, null);

							} else {
								// The switch is turned off
								publisherSession.close();
								publisherSession = null;
							}
						}
					});


					// Subscriber Button
					appContainer.getHomeViewModel().getSwitchSubscribe().setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
						// Subscriber session
						DiscoverySession subscriberSession = null;

						@Override
						public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {
							// Use a switch statement to handle the on and off states
							if (isChecked) {
								SubscribeConfig config = new SubscribeConfig.Builder()
										.setServiceName("Aware_File_Share_Service_Name")
										.build();

								session.subscribe(config, new DiscoverySessionCallback() {

									@Override
									public void onSubscribeStarted(SubscribeDiscoverySession session) {
										Log.d("Subscriber", "Subscriber Started");
										subscriberSession = session;
										appContainer.getHomeViewModel().getSubscriberSessionID().setValue(String.valueOf(session.hashCode()));
									}

									@Override
									public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
										subscriberSession.sendMessage(peerHandle, 0, new byte[0]);
									}

									@Override
									public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
										NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(subscriberSession, peerHandle)
												.setPskPassphrase("somePassword")
												.build();

										NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
												.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
												.setNetworkSpecifier(networkSpecifier)
												.build();

										ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
											@Override
											public void onAvailable(Network network) {
												Log.d("Subscriber Network Manager", "Network available " + network.hashCode());
											}


											@Override
											public void onLost(Network network) {
												Log.d("Subscriber Network Manager", "Network lost " + network.hashCode());
											}
										};

										connectivityManager.requestNetwork(myNetworkRequest, callback);

									}
								}, null);
						} else {
							// The switch is turned off
							subscriberSession.close();
							subscriberSession = null;
						}
				}});


				}

				@Override
				public void onAttachFailed() {
					super.onAttachFailed();

					// Wi-Fi Aware attachment failed
					Log.d("AttachCallback", "Session creation failed");
				}

				@Override
				public void onAwareSessionTerminated(){
					super.onAwareSessionTerminated();
					Log.d("onAwareSessionTerminated", "Session closed, needs to reopen new session");
				}
			}, new IdentityChangedListener() {
				@Override
				public void onIdentityChanged(byte[] mac) {
					super.onIdentityChanged(mac);
				}

				@Override
				public void onClusterIdChanged(int clusterEventType, @NonNull MacAddress clusterId) {
					super.onClusterIdChanged(clusterEventType, clusterId);
					switch (clusterEventType){
						case CLUSTER_CHANGE_EVENT_STARTED:
							Log.d("Cluster","Cluster STARTED - " + clusterId);
							//appContainer.getHomeViewModel().getClusterID().setValue(clusterId.toString());
							break;
						case CLUSTER_CHANGE_EVENT_JOINED:
							Log.d("Cluster","Cluster JOINED - " + clusterId);
							//appContainer.getHomeViewModel().getClusterID().setValue(clusterId.toString());
							break;
					}
				}
			}, workerHandle);
		}
	}

	/**
	 * Stops Wi-Fi Aware and destroys all resources created by it
	 */
	public void closeSessions(){
		if(mWifiAwareSession != null){
			mWifiAwareSession.close();
			mWifiAwareSession = null;
		}
	}
}
