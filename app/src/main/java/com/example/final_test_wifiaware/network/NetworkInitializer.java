package com.example.final_test_wifiaware.network;

import static android.net.wifi.aware.PublishConfig.PUBLISH_TYPE_SOLICITED;
import static android.net.wifi.aware.PublishConfig.PUBLISH_TYPE_UNSOLICITED;
import static android.net.wifi.aware.SubscribeConfig.SUBSCRIBE_TYPE_ACTIVE;
import static com.example.final_test_wifiaware.network.protocol.MessageType.FOUND_PEER;
import static com.example.final_test_wifiaware.network.protocol.MessageType.START_CONNECTION;

import android.annotation.SuppressLint;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.net.ConnectivityManager;
import android.net.LinkProperties;
import android.net.MacAddress;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.Characteristics;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareDataPathSecurityConfig;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.IBinder;
import android.util.Log;
import android.widget.CompoundButton;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.annotation.RequiresApi;


import com.example.final_test_wifiaware.network.connection.PublisherServer;
import com.example.final_test_wifiaware.network.metadata.PublisherMetadata;
import com.example.final_test_wifiaware.network.metadata.SubscriberMetadata;
import com.example.final_test_wifiaware.network.protocol.Client;
import com.example.final_test_wifiaware.network.utils.ClusterIdTracker;
import com.example.final_test_wifiaware.network.utils.IPv6AddressWithPort;
import com.example.final_test_wifiaware.network.utils.PingLinkLocalDevicesTask;
import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.io.IOException;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.Inet6Address;
import java.net.ServerSocket;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.prefs.Preferences;

import kotlinx.coroutines.GlobalScope;

import com.example.final_test_wifiaware.network.wifi_p2p_backup.NetworkInitializerBackup;

@SuppressLint("MissingPermission")
public class NetworkInitializer {
	private static final String TAG = "NetworkInitializer";

	private final Context context;
	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;
	private UUID myUUID;
	private ConnectivityManager connectivityManager;
	private List<Client> subscriberClientList;
	private PublishDiscoverySession publishSession;
	private SubscribeDiscoverySession subscribeSession;
	private HandlerThread worker;
	private Handler workerHandle;
	private Integer messageID = 0;
	private static volatile boolean serverRunning = true;

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

		this.connectivityManager =  (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);

		// Start wifiAwareManager
		//startWifiAwareManager();

		// Create and start a worker thread for background tasks
		worker = new HandlerThread("Worker");
		worker.start();
		workerHandle = new Handler(worker.getLooper());

		// Initialize the list to store subscriber client instances
		subscriberClientList = new ArrayList<>();

		// Start WifiAwareSession
		//createWifiAwareSession();
		//createWifiAwareTest();

		meshTest();
	}


	public void meshTest(){
		MeshTest node = new MeshTest(context);
		node.getMyNode();
	}


	/*public void createWifiAwareTest(){

		WifiAwareManager wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		if (wifiAwareManager.isAvailable()) {

			wifiAwareManager.attach(new AttachCallback(){

				@Override
				public void onAttached(WifiAwareSession session) {
					super.onAttached(session);

					 //Publisher

					PublishConfig publishConfig = new PublishConfig.Builder()
							.setServiceName("test")
							.build();

					session.publish(publishConfig, new DiscoverySessionCallback() {
						private PublishDiscoverySession publisherSession;

						@Override
						public void onPublishStarted(@NonNull PublishDiscoverySession session) {
							super.onPublishStarted(session);
							this.publisherSession = session;
							Log.d("Publisher", "Publisher started");

							 // Server

							ServerSocket ss;
							try {
								ss = new ServerSocket(0);
							} catch (IOException e) {
								throw new RuntimeException(e);
							}

							int port = ss.getLocalPort();

							// start new thread listening to udp packets
							new Thread(() -> {
								try {
									DatagramSocket socket = new DatagramSocket(port);
									byte[] buffer = new byte[1024];
									while(true){
										DatagramPacket packet = new DatagramPacket(buffer, buffer.length);
										socket.receive(packet);
										String message = new String(packet.getData(), 0, packet.getLength());
										System.out.println("Received from " + packet.getAddress().getHostAddress() + ": " + message);
									}
								} catch(Exception e){
									System.err.println("Error: " + e.getMessage());
								}
							}).start();


							NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(session)
									.setPskPassphrase("somePassword")
									.setPort(port)
									.build();
							NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
									.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
									.setNetworkSpecifier(networkSpecifier)
									.build();

							ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
								@Override
								public void onAvailable(@NonNull Network network) {
									super.onAvailable(network);
									Log.d("Pub Network", "Network available: " + network.toString());
								}

								@Override
								public void onLost(@NonNull Network network) {
									super.onLost(network);
									Log.d("Pub Network", "Network lost: " + network.toString());
								}

								@Override
								public void onCapabilitiesChanged(@NonNull Network network, @NonNull NetworkCapabilities networkCapabilities) {
									super.onCapabilitiesChanged(network, networkCapabilities);
									Log.d("Pub Network", "Network capabilities changed for: " + network.toString());
									// You can also log specific capabilities if needed
									Log.d("Pub Network", "New capabilities: " + networkCapabilities.toString());
								}

								@Override
								public void onLinkPropertiesChanged(@NonNull Network network, @NonNull LinkProperties linkProperties) {
									super.onLinkPropertiesChanged(network, linkProperties);
									Log.d("Pub Network", "Link properties changed for: " + network.toString());
									// You can also log specific properties if needed
									Log.d("Pub Network", "New properties: " + linkProperties.toString());
								}
							};

							connectivityManager.requestNetwork(myNetworkRequest, callback);
						}



						@Override
						public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
							super.onMessageReceived(peerHandle, message);
						}
					}, null);


					 // Subscriber



					SubscribeConfig subscribeConfig = new SubscribeConfig.Builder()
							.setServiceName("test")
							.build();

					session.subscribe(subscribeConfig, new DiscoverySessionCallback() {
						private SubscribeDiscoverySession subscriberSession;
						private ScheduledExecutorService executor;
						private Set<IPv6AddressWithPort> ipv6Set = new HashSet<>();
						@Override
						public void onSubscribeStarted(@NonNull SubscribeDiscoverySession session) {
							super.onSubscribeStarted(session);
							this.subscriberSession = session;
							Log.d("Subscriber", "Subscriber started");

							// Schedule the UDP client to run every 5 seconds
							executor = Executors.newSingleThreadScheduledExecutor();
							executor.scheduleAtFixedRate(() -> {
								try {
									DatagramSocket clientSocket = new DatagramSocket();
									byte[] sendData = "Hello".getBytes();

									for (IPv6AddressWithPort clientInfo : ipv6Set) {
										DatagramPacket sendPacket = new DatagramPacket(sendData, sendData.length, clientInfo.getAddress(), clientInfo.getPort());
										clientSocket.send(sendPacket);
									}


									clientSocket.close();
								} catch (IOException e) {
									e.printStackTrace();
								}
							}, 0, 5, TimeUnit.SECONDS); // Run every 5 seconds

						}

						@Override
						public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
							super.onServiceDiscovered(peerHandle, serviceSpecificInfo, matchFilter);
							Log.d("Subscriber", "Service Discovered - Peer: " + peerHandle.hashCode());

							NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(subscriberSession, peerHandle)
									.setPskPassphrase("somePassword")
									.build();
							NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
									.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
									.setNetworkSpecifier(networkSpecifier)
									.build();

							ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
								@Override
								public void onAvailable(@NonNull Network network) {
									super.onAvailable(network);
									Log.d("Sub Network", "Network available: " + network.toString());
								}

								@Override
								public void onLost(@NonNull Network network) {
									super.onLost(network);
									Log.d("Sub Network", "Network lost: " + network.toString());
								}

								@Override
								public void onCapabilitiesChanged(@NonNull Network network, @NonNull NetworkCapabilities networkCapabilities) {
									super.onCapabilitiesChanged(network, networkCapabilities);
									Log.d("Sub Network", "Network capabilities changed for: " + network.toString());
									// You can also log specific capabilities if needed
									Log.d("Sub Network", "New capabilities: " + networkCapabilities.toString());
									WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilities.getTransportInfo();

									Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
									int peerPort = peerAwareInfo.getPort();

									ipv6Set.add(new IPv6AddressWithPort(peerIpv6, peerPort)); // Replace 5000 with your desired port


								}

								@Override
								public void onLinkPropertiesChanged(@NonNull Network network, @NonNull LinkProperties linkProperties) {
									super.onLinkPropertiesChanged(network, linkProperties);
									Log.d("Sub Network", "Link properties changed for: " + network.toString());
									// You can also log specific properties if needed
									Log.d("Sub Network", "New properties: " + linkProperties.toString());
								}
							};

							connectivityManager.requestNetwork(myNetworkRequest, callback);

						}

						@Override
						public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
							super.onMessageReceived(peerHandle, message);
						}
					}, null);

				}

				@Override
				public void onAttachFailed() {
					super.onAttachFailed();
				}

				@Override
				public void onAwareSessionTerminated() {
					super.onAwareSessionTerminated();
				}
			},null);

		} else {
			Log.d(TAG,"WifiAwareManager not available");
		}

	}*/



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

								// Configuring and starting the publication session
								PublishConfig publishConfig = new PublishConfig.Builder()
										.setServiceName("serviceName")
										//.setMatchFilter(Collections.singletonList(Utils.asBytes(myUUID)))
										.setServiceSpecificInfo(Utils.asBytes(myUUID))
										.build();

								mWifiAwareSession.publish(publishConfig, new DiscoverySessionCallback() {

									@Override
									public void onPublishStarted(@NonNull PublishDiscoverySession session) {
										Log.d("Publish","Publisher Started");
										// Publication session started successfully
										publishSession = session;
										AppContainer.getInstance().getHomeViewModel().updatePublisherSessionID("seasonId: " + session.hashCode());
										AppContainer.getInstance().getHomeViewModel().updateSessionPub("" + wifiAwareManager.getAvailableAwareResources().getAvailablePublishSessionsCount());
										// Example security configuration
										WifiAwareDataPathSecurityConfig securityConfig = new WifiAwareDataPathSecurityConfig.Builder(Characteristics.WIFI_AWARE_CIPHER_SUITE_NCS_SK_128)
												.setPskPassphrase("!9WDp4EU!#ax8Cv$hk*3Ca")
												.build();

										// Build network specifier for the connection
										WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(session)
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
												Log.d("PublishNetworkCallback", "Network is now available: " + network.toString());
												AppContainer.getInstance().getHomeViewModel().updateDataPaths("" + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
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
												AppContainer.getInstance().getHomeViewModel().updateDataPaths("" + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
											}

										};
										Log.d("Publisher Request", networkRequest.toString());
										connectivityManager.requestNetwork(networkRequest, callback);
									}

									@Override
									public void onSessionConfigFailed() {
										// Handling failed session configuration
									}

									@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
									@Override
									public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
										//Log.d("onMessageReceived", String.format("Publisher: Received Message from: %s", peerHandle.hashCode()));

										if(Arrays.equals(message, FOUND_PEER)){

											/*// Example security configuration
											WifiAwareDataPathSecurityConfig securityConfig = new WifiAwareDataPathSecurityConfig.Builder(Characteristics.WIFI_AWARE_CIPHER_SUITE_NCS_SK_128)
													.setPskPassphrase("!9WDp4EU!#ax8Cv$hk*3Ca")
													.build();

											// Build network specifier for the connection
											WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(publishSession,peerHandle)
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
													Log.d("PublishNetworkCallback", "Network is now available: " + network.toString());
													AppContainer.getInstance().getHomeViewModel().updateDataPaths("" + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
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
													AppContainer.getInstance().getHomeViewModel().updateDataPaths("" + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
												}

											};
											Log.d("Publisher Request", networkRequest.toString());
											connectivityManager.requestNetwork(networkRequest, callback);*/
											//Log.d("publishSession Rec mess", "sending message with id " + messageID + " to " + peerHandle.hashCode());
											publishSession.sendMessage(peerHandle, messageID++, START_CONNECTION);


										}
									}
								}, workerHandle);

								Toast.makeText(context, "Publisher is ON", Toast.LENGTH_SHORT).show();
							} else {
								mWifiAwareSession.close();
								createWifiAwareSession();

								Toast.makeText(context, "Publisher is OFF", Toast.LENGTH_SHORT).show();
							}
						}
					});
					//AppContainer.getInstance().getSwitchPublisher().setChecked(true);

					// Set an OnCheckedChangeListener to the Switch
					AppContainer.getInstance().getSwitchSubscribe().setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {
						@Override
						public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {

							if (isChecked) {

								// Configuring and starting the subscription session
								SubscribeConfig subscribeConfig = new SubscribeConfig.Builder()
										.setServiceName("serviceName")
										//.setMatchFilter(Collections.singletonList(Utils.asBytes(myUUID)))
										.build();

								mWifiAwareSession.subscribe(subscribeConfig, new DiscoverySessionCallback() {

									@Override
									public void onSubscribeStarted(@NonNull SubscribeDiscoverySession session) {
										Log.d("Subscribe","Subscriber Started");
										// Subscription session started successfully
										subscribeSession = session;
										AppContainer.getInstance().getHomeViewModel().updateSubscriberSessionID("seasonId: " + session.hashCode());
										AppContainer.getInstance().getHomeViewModel().updateSessionSub("" + wifiAwareManager.getAvailableAwareResources().getAvailableSubscribeSessionsCount());

									}

									@Override
									public void onSessionConfigFailed() {
										// Handling failed session configuration for subscription
										subscribeSession = null;
									}

									@Override
									public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
										Log.d("onServiceDiscovered", "sending message with id " + messageID + " to " + peerHandle.hashCode());
										subscribeSession.sendMessage(peerHandle,messageID++,FOUND_PEER);

										// Check if the discovered service's UUID is less than the local UUID
										/*if (Utils.asUuid(serviceSpecificInfo).compareTo(myUUID) < 0) {
											subscribeSession.sendMessage(peerHandle,0,FOUND_PEER);
										}*/

									}

									@RequiresApi(api = Build.VERSION_CODES.TIRAMISU)
									@Override
									public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
										// Handling received messages for subscription and creating client instances
										//Log.d("onMessageReceived", String.format("Subscribe: Received Message from: %s",peerHandle.hashCode()));

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
													.setNetworkSpecifier(networkSpecifier)
													.build();

											ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
												@Override
												public void onAvailable(Network network) {
													// Add log statement for onAvailable
													Log.d("SubscribeNetworkCallback", "Network is now available: " + network.toString());

													AppContainer.getInstance().getHomeViewModel().updateDataPaths("" + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
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
													AppContainer.getInstance().getHomeViewModel().updateDataPaths("" + wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount());
												}

											};

											Log.d("Subscriber Request", networkRequest.toString());

											// Request a network with the specified network request and register network callback
											connectivityManager.requestNetwork(networkRequest, callback);
										}

									}

									@Override
									public void onMessageSendSucceeded(int messageId) {
										// Handling successful message send
										//Log.d("onMessageSendSucceeded", "sending message with id " + (messageID-1));

									}
								}, workerHandle);

								Toast.makeText(context, "Subscribe is ON", Toast.LENGTH_SHORT).show();
							} else {
								mWifiAwareSession.close();
								createWifiAwareSession();
								AppContainer.getInstance().getHomeViewModel().updateSubscriberSessionID("seasonId: -");
								AppContainer.getInstance().getHomeViewModel().updateClusterID("-");

								Toast.makeText(context, "Subscribe is OFF", Toast.LENGTH_SHORT).show();
							}
						}
					});

					//AppContainer.getInstance().getSwitchSubscribe().setChecked(true);

/*					AppContainer.getInstance().getSwitchPingAll().setOnCheckedChangeListener(new CompoundButton.OnCheckedChangeListener() {

						@Override
						public void onCheckedChanged(CompoundButton buttonView, boolean isChecked) {
							PingLinkLocalDevicesTask pingTask = new PingLinkLocalDevicesTask();

							if (isChecked) {
								pingTask.execute();
							}else{
								pingTask.stopScanning();
							}


						}
					});*/
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
