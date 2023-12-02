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
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Message;
import android.util.Log;

import androidx.annotation.NonNull;
import com.example.final_test_wifiaware.network.protocol.Client;
import com.example.final_test_wifiaware.network.protocol.Server;

import com.example.final_test_wifiaware.utils.AppContainer;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;

@SuppressLint("MissingPermission")
public class NetworkInitializer {
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
	private Client subscriberClient;
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

		// Start WifiAwareSession
		createWifiAwareSession();

		worker = new HandlerThread("Worker");
		worker.start();
		workerHandle = new Handler(worker.getLooper());
	}

	/**
	 * Initialize Wi-Fi Aware Manager
	 */
	private void startWifiAwareManager() {
		wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			wifiAwareManagerStatus = true;
			Log.d("wifiAwareManager", "available - getAvailableDataPathsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount()
					+ " - getAvailableSubscribeSessionsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount()
					+ " - getAvailablePublishSessionsCount() = " + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());
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
					mWifiAwareSession = session;
					Log.d("AttachCallback", "Session creation successfully");

					PublishConfig publishConfig = new PublishConfig.Builder().setServiceName("serviceName")
							.setServiceSpecificInfo(asBytes(myUUID)).build();

					session.publish(publishConfig, new DiscoverySessionCallback(){
						@Override
						public void onPublishStarted(@NonNull PublishDiscoverySession session) {
								publishSession = session;
								try {
									publishServer = new Server(connectivityManager);
									publishServer.start();
								} catch (IOException e) {
									session.close();
									publishSession = null;
								}
							}

						@Override
						public void onSessionConfigFailed() {

						}

						@Override
						public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
							boolean connectionAttemptStatus = publishServer.addNewConnection(publishSession, peerHandle);
							Log.d("wifiAwareManager", "available - getAvailableDataPathsCount() = "
									+ wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
							Log.d("Publisher onMessageReceived",String.format(
									"From: %s \\s Attempt Connection Status: %b",
									new String(message),
									connectionAttemptStatus ));

							if (connectionAttemptStatus)
								publishSession.sendMessage(peerHandle,parseInt(new String(message)),message);
						}

					}, workerHandle);

					SubscribeConfig subscribeConfig = new SubscribeConfig.Builder().setServiceName("serviceName").build();
					session.subscribe(subscribeConfig, new DiscoverySessionCallback(){
						private int messageID;
						private Map<Integer, PeerHandle> peerHandleMessageMap;
						@Override
						public void onSubscribeStarted(@NonNull SubscribeDiscoverySession session) {
								subscribeSession = session;
								messageID = 0;
								peerHandleMessageMap = new HashMap<>();
						}

						@Override
						public void onSessionConfigFailed() {
								subscribeSession = null;
						}

						@Override
						public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {

							Log.d("uuid compare",
									String.format("%s to %s is %d",
									asUuid(serviceSpecificInfo), myUUID,
									asUuid(serviceSpecificInfo).compareTo(myUUID)));

							if(asUuid(serviceSpecificInfo).compareTo(myUUID) < 0){
								peerHandleMessageMap.put(messageID,peerHandle);
								subscribeSession.sendMessage(peerHandle, messageID, String.valueOf(messageID).getBytes());
								messageID++;
							}

						}

						@Override
						public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
							subscriberClient = new Client(connectivityManager, subscribeSession, peerHandleMessageMap.get(parseInt(new String(message))));
						}

						@Override
						public void onMessageSendSucceeded(int messageId) {
							Log.d("Message send",
									String.format("%d %d",messageId, peerHandleMessageMap.get(messageId).hashCode()));
						}


					}, workerHandle);

				}

				@Override
				public void onAttachFailed() {
					// Wi-Fi Aware attachment failed
					Log.d("AttachCallback", "Session creation failed");
				}
			}, workerHandle);
		}
	}

	public static UUID asUuid(byte[] bytes) {
		ByteBuffer bb = ByteBuffer.wrap(bytes);
		long firstLong = bb.getLong();
		long secondLong = bb.getLong();
		return new UUID(firstLong, secondLong);
	}
	public static byte[] asBytes(UUID uuid) {
		ByteBuffer bb = ByteBuffer.wrap(new byte[16]);
		bb.putLong(uuid.getMostSignificantBits());
		bb.putLong(uuid.getLeastSignificantBits());
		return bb.array();
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
			subscriberClient.stop();
			subscriberClient = null;
			subscribeSession = null;
		}
		if(mWifiAwareSession != null){
			mWifiAwareSession.close();
			mWifiAwareSession = null;
		}
	}




	// Getters and Setters

}
