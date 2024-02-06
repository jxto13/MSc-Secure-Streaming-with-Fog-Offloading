package com.example.final_test_wifiaware.network.wifi_p2p_backup;

import static android.os.Looper.getMainLooper;

import android.annotation.SuppressLint;
import android.content.Context;
import android.content.IntentFilter;
import android.net.ConnectivityManager;
import android.net.wifi.p2p.WifiP2pManager;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.Log;

import com.example.final_test_wifiaware.network.protocol.Client;
import com.example.final_test_wifiaware.network.protocol.Server;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.List;
import java.util.UUID;

@SuppressLint("MissingPermission")
public class NetworkInitializerBackup {
	private static final String TAG = "NetworkInitializer";

	private final Context context;
	private UUID myUUID;
	private ConnectivityManager connectivityManager;
	private Server publishServer;
	private List<Client> subscriberClientList;
	private HandlerThread worker;
	private Handler workerHandle;

	WifiP2pManager manager;
	WifiP2pManager.Channel channel;
	WiFiDirectBroadcastReceiver receiver;


	public NetworkInitializerBackup(Context context) {
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

		startWifiP2P();
		discoverPeers();
	}


	public void startWifiP2P(){
		PCLNewsAgency observable = new PCLNewsAgency();
		PCLNewsChannel observer = new PCLNewsChannel();

		observable.addPropertyChangeListener(observer);

		manager = (WifiP2pManager) context.getSystemService(Context.WIFI_P2P_SERVICE);
		channel = manager.initialize(context, getMainLooper(), null);
		receiver = new WiFiDirectBroadcastReceiver(manager, channel, observable);

		IntentFilter filter = new IntentFilter();
		filter.addAction(WifiP2pManager.WIFI_P2P_STATE_CHANGED_ACTION);
		filter.addAction(WifiP2pManager.WIFI_P2P_PEERS_CHANGED_ACTION);
		filter.addAction(WifiP2pManager.WIFI_P2P_CONNECTION_CHANGED_ACTION);
		filter.addAction(WifiP2pManager.WIFI_P2P_THIS_DEVICE_CHANGED_ACTION);

		context.registerReceiver(receiver, filter);


	}

	public void discoverPeers() {
		Log.d(TAG, "Initiating peer discovery...");

		manager.discoverPeers(channel, new WifiP2pManager.ActionListener() {
			@Override
			public void onSuccess() {
				Log.d(TAG, "Peer discovery initiated successfully");
				// You can add further actions or logging here upon successful discovery
			}

			@Override
			public void onFailure(int reasonCode) {
				Log.e(TAG, "Peer discovery initiation failed. Reason code: " + reasonCode);

				// You might want to handle specific failure scenarios based on the reasonCode
				switch (reasonCode) {
					case WifiP2pManager.P2P_UNSUPPORTED:
						Log.e(TAG, "Peer-to-peer not supported on this device.");
						break;
					case WifiP2pManager.BUSY:
						Log.e(TAG, "WiFi P2P is busy. Try again later.");
						break;
					case WifiP2pManager.ERROR:
						Log.e(TAG, "Internal error occurred while initiating peer discovery.");
						break;
					default:
						Log.e(TAG, "Unknown error during peer discovery.");
						break;
				}
			}
		});
	}




	// Getters and Setters

}
