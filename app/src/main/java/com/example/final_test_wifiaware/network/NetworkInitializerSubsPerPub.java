package com.example.final_test_wifiaware.network;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.MacAddress;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.Characteristics;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.util.Log;
import android.widget.CompoundButton;
import android.widget.Toast;

import androidx.annotation.RequiresApi;

import com.example.final_test_wifiaware.network.connection.PublisherServer;
import com.example.final_test_wifiaware.network.metadata.PublisherMetadata;
import com.example.final_test_wifiaware.network.metadata.SubscriberMetadata;
import com.example.final_test_wifiaware.network.protocol.Client;
import com.example.final_test_wifiaware.network.utils.ClusterIdTracker;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

@SuppressLint("MissingPermission")
public class NetworkInitializerSubsPerPub {
	private static final String TAG = "NetworkInitializer";

	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	private UUID myUUID;
	private ConnectivityManager connectivityManager;
	private List<Client> subscriberClientList;
	private HandlerThread worker;
	private Handler workerHandle;
	private static volatile boolean serverRunning = true;

	public NetworkInitializerSubsPerPub(Context context) {
		this.context = context;
	}

	/**
	 * Entrypoint for the NetworkInitializer
	 */
	public void start() {
		this.myUUID = AppContainer.getInstance().getMyUUID();

		// Set UUID to the UI
		AppContainer.getInstance().getHomeViewModel().updateMyUUID(myUUID.toString());

		this.connectivityManager =  (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);

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
			wifiAwareManager.attach(new AttachCallback() {
				@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
				@Override
				public void onAttached(WifiAwareSession session) {
					// Wi-Fi Aware session successfully attached
					mWifiAwareSession = session;
					Log.d("AttachCallback", "Session creation successfully");
					new PublisherServer(context,mWifiAwareSession,workerHandle).start();
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

				@RequiresApi(api = Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
				@Override
				public void onStableClusterId(MacAddress stableClusterId) {
					// Your custom logic with stableClusterId
					Log.d("CustomLogic", "Stable clusterId: " + stableClusterId);
					AppContainer.getInstance().getHomeViewModel().updateClusterID("" + stableClusterId);

					// Set an OnCheckedChangeListener to the Switch
					AppContainer.getInstance().getSwitchPublisher().setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
						@Override
						public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {
							if (isChecked) {
								new PublisherMetadata(context,mWifiAwareSession, workerHandle).start();

								Toast.makeText(context, "Publisher is ON", Toast.LENGTH_SHORT).show();
							} else {
								mWifiAwareSession.close();
								createWifiAwareSession();

								Toast.makeText(context, "Publisher is OFF", Toast.LENGTH_SHORT).show();
							}
						}
					});


					// Set an OnCheckedChangeListener to the Switch
					AppContainer.getInstance().getSwitchSubscribe().setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
						@Override
						public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {

							if (isChecked) {
								new SubscriberMetadata(context,mWifiAwareSession, workerHandle).start();


								Toast.makeText(context, "Subscribe is ON", Toast.LENGTH_SHORT).show();
							} else {
								mWifiAwareSession.close();
								createWifiAwareSession();

								Toast.makeText(context, "Subscribe is OFF", Toast.LENGTH_SHORT).show();
							}
						}
					});
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




	// Getters and Setters

}
