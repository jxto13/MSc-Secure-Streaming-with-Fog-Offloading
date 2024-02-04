package com.example.final_test_wifiaware.network;

import static java.lang.Integer.parseInt;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareSession;
import android.nfc.Tag;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Message;
import android.util.Log;

import androidx.annotation.NonNull;
import com.example.final_test_wifiaware.network.protocol.Client;
import com.example.final_test_wifiaware.network.protocol.Server;

import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
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
	@SuppressLint("DefaultLocale")
	private void startWifiAwareManager() {
		wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			wifiAwareManagerStatus = true;

			Characteristics characteristics = wifiAwareManager.getCharacteristics();

			if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {

				assert characteristics != null;
				Log.d(TAG,String.format(
						"isSetChannelOnDataPathSupported() - %b \n" +
						"getNumberOfSupportedDataInterfaces - %d \n" +
						"getNumberOfSupportedDataPaths - %d \n",
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
			wifiAwareManager.attach(new AttachCallback() {
				@Override
				public void onAttached(WifiAwareSession session) {
					// Wi-Fi Aware session successfully attached
					mWifiAwareSession = session;
					Log.d("AttachCallback", "Session creation successfully");

					// Configuring and starting the publication session
					PublishConfig publishConfig = new PublishConfig.Builder()
							.setServiceName("serviceName")
							.setServiceSpecificInfo(Utils.asBytes(myUUID))
							.build();

					session.publish(publishConfig, new DiscoverySessionCallback() {
						private int messageID;

						@Override
						public void onPublishStarted(@NonNull PublishDiscoverySession session) {
							// Publication session started successfully
							publishSession = session;
							messageID = 0;

							AppContainer.getInstance().getHomeViewModel().updatePublisherSessionID("seasonId: " + session.hashCode());


							// Starting the server for handling incoming connections
							try {
								publishServer = new Server(connectivityManager);
								publishServer.start();
							} catch (IOException e) {
								// Closing the session if server start fails
								session.close();
								publishSession = null;
							}
						}

						@Override
						public void onSessionConfigFailed() {
							// Handling failed session configuration
						}

						@Override
						public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
							// Processing received messages and managing connections
							boolean connectionAttemptStatus = publishServer.addNewConnection(publishSession, peerHandle);

							Log.d("onMessageReceived", String.format("Publisher: Received Message from: %s", Utils.asUuid(message)));

							// Sending a response if connection attempt was successful
							if (connectionAttemptStatus)
								publishSession.sendMessage(peerHandle, messageID++, Utils.asBytes(myUUID));
						}
					}, workerHandle);

					// Configuring and starting the subscription session
					SubscribeConfig subscribeConfig = new SubscribeConfig.Builder()
							.setServiceName("serviceName")
							.build();

					session.subscribe(subscribeConfig, new DiscoverySessionCallback() {
						private int messageID;
						private Map<UUID, PeerHandle> peerHandleMessageMap;

						@Override
						public void onSubscribeStarted(@NonNull SubscribeDiscoverySession session) {
							// Subscription session started successfully
							subscribeSession = session;
							messageID = 0;
							peerHandleMessageMap = new HashMap<>();

							AppContainer.getInstance().getHomeViewModel().updateSubscriberSessionID("seasonId: " + session.hashCode());

						}

						@Override
						public void onSessionConfigFailed() {
							// Handling failed session configuration for subscription
							subscribeSession = null;
						}

						@Override
						public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
							// Handling the discovery of a service and initiating communication
							Log.d("uuid compare", String.format("%s to %s is %d",
									Utils.asUuid(serviceSpecificInfo),
									myUUID,
									Utils.asUuid(serviceSpecificInfo).compareTo(myUUID)));

							// Check if the discovered service's UUID is less than the local UUID
							if (Utils.asUuid(serviceSpecificInfo).compareTo(myUUID) < 0) {
								// Store the PeerHandle associated with the discovered UUID in the map
								peerHandleMessageMap.put(Utils.asUuid(serviceSpecificInfo), peerHandle);

								// Send a message to the discovered peer with a unique message ID and the local UUID
								subscribeSession.sendMessage(peerHandle, messageID, Utils.asBytes(myUUID));

								// Log the successful message send to the specified peer
								Log.d("onMessageSendSucceeded", String.format("Subscribe: Sent Message to %s", myUUID));

								// Increment the message ID for the next message
								messageID++;
							}

						}

						@Override
						public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
							// Handling received messages for subscription and creating client instances
							subscriberClientList.add(new Client(connectivityManager, subscribeSession, peerHandleMessageMap.get(Utils.asUuid(message))));
							Log.d("onMessageReceived", String.format("Subscribe: Received Message from: %s", Utils.asUuid(message)));
						}

						@Override
						public void onMessageSendSucceeded(int messageId) {
							// Handling successful message send
						}
					}, workerHandle);

				}

				@Override
				public void onAttachFailed() {
					// Wi-Fi Aware attachment failed
					Log.d("AttachCallback", "Session creation failed");
				}
			},new IdentityChangedListenerCallbacks(), workerHandle);
		}
	}




	/**
	 * Stops Wi-Fi Aware and destroys all resources created by it
	 */

	public void closeSessions(){
		if(publishSession != null){
			publishSession.close();
			publishServer.stop();
			publishServer = null;
			publishSession = null;
		}
		if(subscribeSession != null){
			subscribeSession.close();
			for (Client client : subscriberClientList ) {
				client.stop();
				client = null;
			}
			subscribeSession = null;
		}
		if(mWifiAwareSession != null){
			mWifiAwareSession.close();
			mWifiAwareSession = null;
		}
	}




	// Getters and Setters

}
