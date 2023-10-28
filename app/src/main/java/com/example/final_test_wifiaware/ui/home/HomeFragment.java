package com.example.final_test_wifiaware.ui.home;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Bundle;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;

import androidx.annotation.NonNull;
import androidx.core.app.ActivityCompat;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import com.example.final_test_wifiaware.R;
import com.example.final_test_wifiaware.databinding.FragmentHomeBinding;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.DatagramPacket;
import java.net.DatagramSocket;
import java.net.Inet6Address;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.SocketException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.concurrent.TimeUnit;

public class HomeFragment extends Fragment {

	private FragmentHomeBinding binding;
	private HomeViewModel homeViewModel;

	private WifiAwareManager wifiAwareManager;
	private boolean wifiAwareManagerStatus;
	private WifiAwareSession mWifiAwareSession;

	private PublishDiscoverySession publishDiscoverySession;

	private SubscribeDiscoverySession subscribeDiscoverySession;

	private DiscoverySessionCallback discoverySessionCallback;

	private List<PeerHandle> connectedClients = new ArrayList<>();

	private PeerHandle publisherPeerHandle;

	private NetworkCapabilities networkCapabilitiesSocket;
	private Network networkPeer;
	private Network subscriberNetworkPeer;

	private ArrayAdapter<PeerHandle> adapter;

	private Spinner spinner;

	/**
	 * Initialize Wi-Fi Aware manager
	 */
	public void startWifiAwareManager() {
		wifiAwareManager = (WifiAwareManager) this.getContext().getSystemService(Context.WIFI_AWARE_SERVICE);

		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			wifiAwareManagerStatus = true;
			Log.d("wifiAwareManager", "available");
		} else {
			wifiAwareManagerStatus = false;
			Log.d("wifiAwareManager", "not available");
			// Wi-Fi Aware is not available on this device
		}
	}

	/**
	 * Create a SubscribeConfig for subscribing to data
	 */
	public PublishConfig createPublishConfig() {
		return new PublishConfig.Builder()
				.setServiceSpecificInfo("Stream Video".getBytes())
				.setServiceName("test") // Set the same service name as in publish
				.build();
	}

	/**
	 * Create a SubscribeConfig for subscribing to data
	 */
	public SubscribeConfig createSubscribeConfig() {
		return new SubscribeConfig.Builder()
				.setServiceName("test")
				.build();
	}

	/**
	 * Create a DiscoverySessionCallback
	 */
	public void startDiscovery() {
		discoverySessionCallback = new DiscoverySessionCallback() {
			@Override
			public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
				// Handle discovered services here

				Log.d("ServiceDiscovered", "Service name: " + new String(serviceSpecificInfo) + " | peerHandle: " + peerHandle.hashCode());

				// Subscribe to the service here using the known service name or any criteria you require.

				subscribeDiscoverySession.sendMessage(peerHandle,1,"subscriverID".getBytes());


			}

			@Override
			public void onPublishStarted(PublishDiscoverySession session) {
				publishDiscoverySession = session;
				Log.d("onPublishStarted", session.toString());

				// Publish session started, you can now send data
			}

			@Override
			public void onSubscribeStarted(SubscribeDiscoverySession session) {
				subscribeDiscoverySession = session;
				// Subscribe session started, you can now receive data
				Log.d("onSubscribeStarted", session.toString());
				//session.sendMessage(1,"send from pixel 3a xl".getBytes());
			}

			@Override
			public void onSessionConfigFailed() {
				Log.e("SessionConfigFailed", "Session configuration failed");
				// Session configuration failed
			}
			@Override
			public void onMessageSendSucceeded(int messageId) {
				Log.d("Message","send successfully");
			}
			@Override
			public void onMessageSendFailed(int messageId) {
				Log.d("Message","send failed");

			}
			@Override
			public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
				Log.d("Message","received from " + peerHandle.hashCode() + ": " + new String(message));
				String messageString = new String(message);
				switch (messageString){
					case "subscriverID":
						adapter.add(peerHandle);
						adapter.notifyDataSetChanged();
						connectedClients.add(peerHandle);
						break;
					case "publisherID":
						publisherPeerHandle = peerHandle;
						initializeSocketConnectionSubscriver();
						break;
				}


			}
		};
	}

	/**
	 * Attach to Wi-Fi Aware and create a session
	 */
	public void createWifiAwareSession() {
		if (wifiAwareManagerStatus) {
			wifiAwareManager.attach(new AttachCallback() {
				@Override
				public void onAttached(WifiAwareSession session) {
					mWifiAwareSession = session;
					Log.d("Custom", "Session creation successfully");
				}

				@Override
				public void onAttachFailed() {
					// Wi-Fi Aware attachment failed
					Log.d("Custom-Error", "Session creation failed");
				}
			}, null);
		}
	}

	public void startPublisher() {
		if (ActivityCompat.checkSelfPermission(requireContext(), Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
			// TODO: Consider calling
			//    ActivityCompat#requestPermissions
			// here to request the missing permissions, and then overriding
			//   public void onRequestPermissionsResult(int requestCode, String[] permissions,
			//                                          int[] grantResults)
			// to handle the case where the user grants the permission. See the documentation
			// for ActivityCompat#requestPermissions for more details.
			return;
		}
		// Create a publisher discovery session
		mWifiAwareSession.publish(createPublishConfig(), discoverySessionCallback, null);

	}

	public void startSubscriver(){
		if (ActivityCompat.checkSelfPermission(requireContext(), Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
			// TODO: Consider calling
			//    ActivityCompat#requestPermissions
			// here to request the missing permissions, and then overriding
			//   public void onRequestPermissionsResult(int requestCode, String[] permissions,
			//                                          int[] grantResults)
			// to handle the case where the user grants the permission. See the documentation
			// for ActivityCompat#requestPermissions for more details.
			return;
		}
		// Create a subscribe discovery session
		mWifiAwareSession.subscribe(createSubscribeConfig(), discoverySessionCallback, null);
	}
	public void stopWifiAware() {
		if (mWifiAwareSession != null) {
			// Close the Wi-Fi Aware session to terminate publish and subscribe sessions
			mWifiAwareSession.close();
		}
	}


	public View onCreateView(@NonNull LayoutInflater inflater,
							 ViewGroup container, Bundle savedInstanceState) {
		binding = FragmentHomeBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		final TextView textView = binding.textHome;
		homeViewModel = new ViewModelProvider(this).get(HomeViewModel.class);

		// Observe the LiveData for text and set it to the TextView.
		homeViewModel.getText().observe(getViewLifecycleOwner(), textView::setText);

		// Start wifiAwareManager
		startWifiAwareManager();

		// Start WifiAwareSession
		createWifiAwareSession();

		// Start Discovery
		startDiscovery();


		Button publishButton = root.findViewById(R.id.startPublishButton);
		publishButton.setOnClickListener(new View.OnClickListener() {
			@Override
			public void onClick(View v) {
				// When the refresh button is clicked, refresh the discovery session.
				startPublisher();
			}
		});

		Button subscriverButton = root.findViewById(R.id.startSubscriverButton);
		subscriverButton.setOnClickListener(new View.OnClickListener() {
			@Override
			public void onClick(View v) {
				// When the refresh button is clicked, refresh the discovery session.
				startSubscriver();
			}
		});

		Button refresh = root.findViewById(R.id.chip);
		refresh.setOnClickListener(new View.OnClickListener() {

			@Override
			public void onClick(View v) {
				// When the refresh button is clicked, refresh the discovery session.
				ScrollView scrollView = root.findViewById(R.id.scrollView);
				LinearLayout peerHandleList = root.findViewById(R.id.peerHandleList);
				// Clear the existing TextViews from the LinearLayout
				peerHandleList.removeAllViews();
				for (PeerHandle peerHandle : connectedClients) {
					// Create a new TextView to display the PeerHandle
					TextView textView = new TextView(getContext());
					textView.setText("PeerHandle: " + peerHandle.hashCode());

					// Add the TextView to the LinearLayout
					peerHandleList.addView(textView);
				}
				//TextView textView = new TextView(getContext());
				//textView.setText("PeerHandle: " );
				//peerHandleList.addView(textView);

				// After adding or removing TextViews, scroll to the top to see the newly added content
				scrollView.fullScroll(ScrollView.FOCUS_UP);
			}
		});

		Button prepareSocket = root.findViewById(R.id.prepareSocket);
		prepareSocket.setOnClickListener(new View.OnClickListener() {
			@Override
			public void onClick(View v) {
				initializeSocketConnectionPublisher();
			}
		});

		Button startSocketTest = root.findViewById(R.id.socketTest);
		startSocketTest.setOnClickListener(new View.OnClickListener() {
			@Override
			public void onClick(View v) {
				createSocket();
			}
		});

		spinner = root.findViewById(R.id.spinner);
		EditText messageInput = root.findViewById(R.id.messageInput);
		Button sendButton = root.findViewById(R.id.sendButton);

		// Create an array of items in code
		List<PeerHandle> peerHandles = new ArrayList<>();

		// Create an ArrayAdapter for the Spinner using the array
		adapter = new ArrayAdapter<>(
				getContext(), android.R.layout.simple_spinner_item, peerHandles);
		adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item);
		spinner.setAdapter(adapter);

		spinner.setPrompt("Select an item");

		sendButton.setOnClickListener(new View.OnClickListener() {
			int counter = 0;
			@Override
			public void onClick(View view) {
				PeerHandle selectedItem = (PeerHandle) spinner.getSelectedItem();
				String message = messageInput.getText().toString();

				// Do something with the selected item and the message (e.g., send it)
				// For this example, we'll just show a Toast message with the selected item and message.
				String toastMessage = "Selected Item: " + selectedItem + "\nMessage: " + message;
				publishDiscoverySession.sendMessage(selectedItem, counter++,toastMessage.getBytes() );
				//Toast.makeText(this, toastMessage, Toast.LENGTH_SHORT).show();
			}
		});


		// Observe the LiveData for discovered devices and update the UI accordingly.
		// Update the UI to display the list of discovered devices.
		// You can use a RecyclerView or other UI elements to show the devices.
		homeViewModel.getDiscoveredDevices().observe(getViewLifecycleOwner(), System.out::println);

		return root;
	}

	public void initializeSocketConnectionPublisher() {
		int counter = 0;
		for (PeerHandle client: connectedClients) {

			// Step 1: Start a ServerSocket on the publisher
			DatagramSocket ss = null;
			try {
				ss = new DatagramSocket(0);
			} catch (SocketException e) {
				throw new RuntimeException(e);
			}


			int port = ss.getLocalPort();


			// Step 2: Create a WifiAwareNetworkSpecifier
			NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(publishDiscoverySession, client)
					.setPskPassphrase("somePassword")
					.setPort(port)
					.build();


			// Step 3: Create a NetworkRequest and set up a network callback
			NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
					.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
					.setNetworkSpecifier(networkSpecifier)
					.build();

			ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
				@Override
				public void onAvailable(Network network) {
					Log.d("NetworkCallback", "Network is available");
				}

				@Override
				public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
					Log.d("NetworkCallback", "Network capabilities have changed");
					subscriberNetworkPeer = network;

				}

				@Override
				public void onLost(Network network) {
					Log.d("NetworkCallback", "Network is lost");
				}
			};


			ConnectivityManager connMgr = (ConnectivityManager) getContext().getSystemService(Context.CONNECTIVITY_SERVICE);
			connMgr.requestNetwork(myNetworkRequest, callback);

			//send message to peer-handler
			Log.d("Message", "Send message to subscriber to start network");
			publishDiscoverySession.sendMessage(client, counter++, "publisherID".getBytes());


			DatagramSocket finalSs = ss;
			Runnable networkTask = () -> {
				while (true) {
					byte[] receiveData = new byte[2048];
					DatagramPacket receivePacket = new DatagramPacket(receiveData, receiveData.length);
					try {
						finalSs.receive(receivePacket);
					} catch (IOException e) {
						throw new RuntimeException(e);
					}

					String message = new String(receivePacket.getData(), 0, receivePacket.getLength());
					InetAddress clientAddress = receivePacket.getAddress();
					int clientPort = receivePacket.getPort();

					DatagramPacket sendPacket = new DatagramPacket(receivePacket.getData(), receivePacket.getLength(), clientAddress, clientPort);
					try {
						finalSs.send(sendPacket);
					} catch (IOException e) {
						throw new RuntimeException(e);
					}
				}
			};

			// Create and start a new thread to run the network task
			Thread thread = new Thread(networkTask);
			thread.start();
		}
	}

	public void initializeSocketConnectionSubscriver() {


		// Step 1: Start a ServerSocket on the publisher
		DatagramSocket ss = null;
		try {
			ss = new DatagramSocket(0);
		} catch (SocketException e) {
			throw new RuntimeException(e);
		}


		// Step 2: Create a WifiAwareNetworkSpecifier
		NetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(subscribeDiscoverySession, publisherPeerHandle)
				.setPskPassphrase("somePassword")
				.build();


		// Step 3: Create a NetworkRequest and set up a network callback
		NetworkRequest myNetworkRequest = new NetworkRequest.Builder()
				.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
				.setNetworkSpecifier(networkSpecifier)
				.build();

		ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
			@Override
			public void onAvailable(Network network) {
				Log.d("NetworkCallback", "Network is available");
			}

			@Override
			public void onCapabilitiesChanged(Network network, NetworkCapabilities networkCapabilities) {
				Log.d("NetworkCallback", "Network capabilities have changed");
				if(networkCapabilities != null){
					Log.d("NetworkCallback", "Network capabilities is" + networkCapabilities.getTransportInfo());
					Log.d("NetworkCallback", "Network UpstreamBandwidthKbps is " + networkCapabilities.getLinkUpstreamBandwidthKbps());
					Log.d("NetworkCallback", "Network DownstreamBandwidthKbps is " + networkCapabilities.getLinkDownstreamBandwidthKbps());
					Log.d("NetworkCallback", "Network SignalStrength is " + networkCapabilities.getSignalStrength());



				}
				networkCapabilitiesSocket = networkCapabilities;
				networkPeer = network;
			}

			@Override
			public void onLost(Network network) {
				Log.d("NetworkCallback", "Network is lost");
			}
		};


		ConnectivityManager connMgr = (ConnectivityManager) getContext().getSystemService(Context.CONNECTIVITY_SERVICE);
		connMgr.requestNetwork(myNetworkRequest, callback);


	}

	private static long sendUDPRequest(DatagramSocket clientSocket, Inet6Address peerIpv6, int serverPort, String message) {
		try {

			byte[] sendData = message.getBytes();
			DatagramPacket sendPacket = new DatagramPacket(sendData, sendData.length, peerIpv6, serverPort);
			DatagramPacket receivePacket = new DatagramPacket(new byte[2048], 2048);

			long startTime = System.currentTimeMillis();
			clientSocket.send(sendPacket);
			clientSocket.receive(receivePacket);
			long endTime = System.currentTimeMillis();

			return endTime - startTime;
		} catch (Exception e) {
			e.printStackTrace();
			return -1;
		}
	}


	public void createSocket() {
		WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) networkCapabilitiesSocket.getTransportInfo();
		assert peerAwareInfo != null;
		Inet6Address peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
		int peerPort = peerAwareInfo.getPort();
		Log.d("createSocket", "peerIpv6:" + peerIpv6 + " | " + networkCapabilitiesSocket.toString());

		Runnable networkTask = () -> {
			/*
			int numPackets = 1000;
			String message = "BenchmarkPacket";


			long startTime = System.currentTimeMillis();
			long totalLatency = 0;
			long totalBytesSent = 0;

			for (int i = 0; i < numPackets; i++) {
				long latency = sendUDPRequest(peerIpv6, peerPort, message);
				totalLatency += latency;
				totalBytesSent += message.length();
			}

			long endTime = System.currentTimeMillis();
			long totalTime = endTime - startTime;

			// Calculate bandwidth in bytes per second
			double bandwidth = (totalBytesSent * 1000.0) / totalTime;
			double averageLatency = (double) totalLatency / numPackets;

			System.out.println("Bandwidth: " + bandwidth / 1048576 + " MB/second");
			System.out.println("Average Latency: " + averageLatency + " ms");*/
			// Define different test parameters (number of packets and packet sizes)
			DatagramSocket clientSocket = null;
			try {
				clientSocket = new DatagramSocket();
			} catch (SocketException e) {
				throw new RuntimeException(e);
			}

			try {
				networkPeer.bindSocket(clientSocket);
			} catch (IOException e) {
				throw new RuntimeException(e);
			}
			int[] packetCounts = {100, 1000 };
			int[] packetSizes = {1500};
			List<TestResult> results = new ArrayList<>();

			for (int numPackets : packetCounts) {
				Log.d("Benchmark:", "numPackets - " + numPackets);
				for (int packetSize : packetSizes) {
					Log.d("Benchmark:", "packetSizes - " + packetSize);

					String message = generateMessage(packetSize);

					long startTime = System.currentTimeMillis();
					long totalLatency = 0;
					long totalBytesSent = 0;

					for (int i = 0; i < numPackets; i++) {
						long latency = sendUDPRequest(clientSocket, peerIpv6, peerPort, message);
						totalLatency += latency;
						totalBytesSent += message.length();
					}

					long endTime = System.currentTimeMillis();
					long totalTime = endTime - startTime;

					// Calculate bandwidth in megabytes per second
					double bandwidthInBytesPerSecond = (totalBytesSent * 1000.0) / totalTime;
					double bandwidthInMegabytesPerSecond = bandwidthInBytesPerSecond / 1048576; // 1 MB = 1,048,576 bytes
					double averageLatency = (double) totalLatency / numPackets;

					// Create a TestResult object and add it to the results list
					results.add(new TestResult(numPackets, packetSize, bandwidthInMegabytesPerSecond, averageLatency));
				}
			}
			System.out.print("[");
			results.forEach((result) -> System.out.print(result+ ","));
			System.out.print("]");

		};

		// Create and start a new thread to run the network task
		Thread thread = new Thread(networkTask);
		thread.start();

	}


	private static String generateMessage(int size) {
		// Create a message of the specified size
		StringBuilder message = new StringBuilder();
		for (int i = 0; i < size; i++) {
			message.append("A");
		}
		return message.toString();
	}

	class TestResult {
		int numPackets;
		int packetSize;
		double bandwidth;
		double averageLatency;

		TestResult(int numPackets, int packetSize, double bandwidth, double averageLatency) {
			this.numPackets = numPackets;
			this.packetSize = packetSize;
			this.bandwidth = bandwidth;
			this.averageLatency = averageLatency;
		}
		@Override
		public String toString() {
			return "{\"numPackets\": " + numPackets +
					",\"packetSize\": " + packetSize +
					",\"bandwidth\":" + bandwidth +
					",\"averageLatency\":" + averageLatency + "}";
		}
	}



	public void closeSocketConnection() {
		// Step 9: Unregister network callback when finished with the connection
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		stopWifiAware();
		binding = null;
	}
}
