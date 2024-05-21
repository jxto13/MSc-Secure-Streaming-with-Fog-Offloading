package pt.iris.network.protocol;

import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.util.Log;

import androidx.annotation.NonNull;


import pt.iris.network.mesh.PingClient;

import java.net.InetAddress;
import java.util.UUID;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;

public class Connection {
	// Custom UUID
	private final UUID uuid;
	// Peer object from Wifi-Aware API
	private final PeerHandle peerHandle;
	// Distance to the Peer - 1000 equals 1m
	private Integer distance;
	// Android ConnectivityManager for managing network requests
	private ConnectivityManager mConManager;
	private DiscoverySession discoverySession;

	private NetworkCapabilities mCurrentNetCapabilities;


	// Enum to represent connection states
	public enum ConnectionState {
		CONNECTED,
		NON_CONNECTED
	}

	// Static variable to hold the current connection state
	private ConnectionState connectionState = ConnectionState.NON_CONNECTED;

	public ConnectionState getConnectionState() {
		return connectionState;
	}

	// Network that Peer belongs, if null means that is not connected
	private Network connectionNetwork;
	public Connection(UUID uuid, PeerHandle peerHandle, int distance, ConnectivityManager manager, DiscoverySession discoverySession) {
		this.uuid = uuid;
		this.peerHandle = peerHandle;
		this.distance = distance;
		this.mConManager = manager;
		this.discoverySession = discoverySession;
	}
	public Connection(PeerHandle peerHandle, ConnectivityManager manager, DiscoverySession discoverySession) {
		this.uuid = null;
		this.peerHandle = peerHandle;
		this.mConManager = manager;
		this.discoverySession = discoverySession;
	}

	@NonNull
	@Override
	public String toString() {
		return String.format("Connection { uuid = %s, connectedClient = %s}" ,
				uuid.toString(), peerHandle.hashCode());
	}

	// Getters and Setters
	public Network getNetwork() {
		return connectionNetwork;
	}

	public PeerHandle getPeerHandle() {
		return peerHandle;
	}

	public void startNetworkFromServer(){
		// Build network specifier for the connection
		WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder((PublishDiscoverySession) discoverySession) // type cast to PublishDiscoverySession
				.setPskPassphrase("testConnection")
				.setPort(8080) // Predefined in the Server ping server
				.build();

		// Build network request for the connection
		NetworkRequest networkRequest = new NetworkRequest.Builder()
				.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
				.setNetworkSpecifier(networkSpecifier)
				.build();

		// Request a network with the specified network request
		mConManager.requestNetwork(networkRequest, new Connection.ServerNetworkCallback());
	}

	public void startNetworkFromClient(){
		// Build network specifier for the connection
		WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(discoverySession, peerHandle)
				.setPskPassphrase("testConnection")
				.build();

		// Build network request for the connection
		NetworkRequest networkRequest = new NetworkRequest.Builder()
				.addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
				.setNetworkSpecifier(networkSpecifier)
				.build();

		// Request a network with the specified network request
		mConManager.requestNetwork(networkRequest, new Connection.ClientNetworkCallback());

	}


	/**
	 * Represents the Server network callback for Wi-Fi Aware.
	 */
	private class ServerNetworkCallback extends ConnectivityManager.NetworkCallback {

		@Override
		public void onAvailable(@NonNull Network network) {
			super.onAvailable(network);
			Log.d("onAvailable", "ServerNetwork: Network Available " + network.hashCode());

			connectionNetwork = network;
			connectionState = ConnectionState.CONNECTED;
		}

		@Override
		public void onLost(@NonNull Network network) {
			super.onLost(network);
			Log.d("onAvailable", "ServerNetwork: Network Lost " + network.hashCode());

			mConManager.unregisterNetworkCallback(this);
			connectionState = ConnectionState.NON_CONNECTED;
		}

	}

	/**
	 * Represents the Server network callback for Wi-Fi Aware.
	 */
	private class ClientNetworkCallback extends ConnectivityManager.NetworkCallback {

		private PingClient pingClient;
		private Thread clientThread;
		private ScheduledExecutorService scheduler;
		@Override
		public void onAvailable(@NonNull Network network) {
			super.onAvailable(network);
			Log.d("onAvailable", "ClientNetwork: Network Available " + network.hashCode());

			connectionNetwork = network;
			connectionState = ConnectionState.CONNECTED;

			// Start sending messages every 5 seconds
			if (scheduler == null || scheduler.isShutdown()) {
				scheduler = Executors.newSingleThreadScheduledExecutor();
				scheduler.scheduleAtFixedRate(() -> {
					if (pingClient != null) {
						pingClient.addMessage("Heart Beat Ping"); // Add your message here
					}
				}, 0, 10, TimeUnit.SECONDS);
			}
		}

		@Override
		public void onLost(@NonNull Network network) {
			super.onLost(network);
			Log.d("onLost", "ClientNetwork: Network Lost " + network.hashCode());

			mConManager.unregisterNetworkCallback(this);
			connectionState = ConnectionState.NON_CONNECTED;

			// Stop sending messages and shutdown the scheduler
			if (scheduler != null && !scheduler.isShutdown()) {
				scheduler.shutdown();
			}

			// Stop the ping client
			if (pingClient != null) {
				pingClient.stop();
			}
		}

		@Override
		public void onCapabilitiesChanged(@NonNull Network network, @NonNull NetworkCapabilities networkCapabilities) {
			super.onCapabilitiesChanged(network, networkCapabilities);
			Log.d("onCapabilitiesChanged", "ClientNetwork: New network " + network.hashCode());

			if (mCurrentNetCapabilities == null) {
				// Set network capabilities and start the client if not already enabled
				mCurrentNetCapabilities = networkCapabilities;

				// Extract information from the network capabilities
				WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) mCurrentNetCapabilities.getTransportInfo();
				InetAddress peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
				int peerPort = 8080;

				pingClient = new PingClient(peerIpv6, peerPort);
				clientThread = new Thread(pingClient);
				clientThread.start();
			} else {
				Log.d("onCapabilitiesChanged", "onCapabilitiesChanged: Different network or new capability");
			}
		}
	}

}