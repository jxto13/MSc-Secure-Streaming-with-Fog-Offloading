package pt.iris.network.mesh;

import static pt.iris.network.protocol.MessageType.*;

import android.annotation.SuppressLint;
import android.content.Context;
import android.net.ConnectivityManager;
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
import android.util.Log;

import androidx.annotation.NonNull;

import pt.iris.network.protocol.Connection;
import pt.iris.network.protocol.NodeType;
import pt.iris.network.utils.Utils;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.Queue;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

public class VirtualNodeWifiAware {
	private final String TAG = "VirtualNodeWifiAware";
	private final Context context;
	private final WifiAwareSession mWifiAwareSession;
	private UUID myUUID;
	// NodeType can be Client Node or Relay Node
	private NodeType nodeType;
	private HandlerThread worker;
	private Handler workerHandle;
	private ConnectivityManager connectivityManager;
	private List<Connection> connections = new ArrayList<>();
	private WifiAwareManager wifiAwareManager;

	//private ConnectionManager connectionManager;
	public VirtualNodeWifiAware(WifiAwareSession session, Context inputContext, UUID myUUID, Integer interfaceCount) {
		this.nodeType = NodeType.fromInterfaceCount(interfaceCount);
		this.mWifiAwareSession = session;
		this.context = inputContext;
		this.myUUID = myUUID;
		this.connectivityManager =  (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
		this.wifiAwareManager = (WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);

		// Create and start a worker thread for background tasks
		worker = new HandlerThread("VirtualNodeWifiAwareWorker");
		worker.start();
		workerHandle = new Handler(worker.getLooper());
	}

	@SuppressLint("MissingPermission")
	public void startRelay(){

		// Configuring and starting the publication session
		PublishConfig publishConfig = new PublishConfig.Builder()
				.setServiceName("serviceName")
				.setServiceSpecificInfo(Utils.asBytes(myUUID))
				.build();

		mWifiAwareSession.publish(publishConfig, new DiscoverySessionCallback() {
			private int messageID = 0;
			private PublishDiscoverySession publishSession;

			private Map<UUID, PeerHandle> peerHandleList = new HashMap<>();

			final PingServer pingServer = new PingServer();
			final Thread serverThread = new Thread(pingServer);


			@Override
			public void onPublishStarted(@NonNull PublishDiscoverySession session) {
				super.onPublishStarted(session);
				Log.d("Publish", "Publisher Started");
				publishSession = session;
				serverThread.start();
			}

			@Override
			public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
				super.onMessageReceived(peerHandle, message);

				if (Arrays.equals(message, FOUND_PEER)) {
					publishSession.sendMessage(peerHandle, messageID++, ACK_PEER);
					Log.d("onMessageReceived", String.format("Publisher: %s from %s", "FOUND_PEER",peerHandle.hashCode()));
				}

				if (Arrays.equals(message, REQUEST_INTERFACE)) {
					Log.d("onMessageReceived", String.format("Publisher: %s from %s", "REQUEST_INTERFACE",peerHandle.hashCode()));

					if(wifiAwareManager.getAvailableAwareResources().getAvailableDataPathsCount() == 0){
						publishSession.sendMessage(peerHandle, messageID++, DENY_INTERFACE);
					}
					publishSession.sendMessage(peerHandle, messageID++, ACK_INTERFACE);
				}

				if (Arrays.equals(message, REQUEST_CONNECTION)) {
					Log.d("onMessageReceived", String.format("Publisher: %s from %s", "REQUEST_CONNECTION",peerHandle.hashCode()));

					Connection tmp = new Connection(peerHandle, connectivityManager, publishSession);
					tmp.startNetworkFromServer();
					connections.add(tmp);
					publishSession.sendMessage(peerHandle, messageID++, START_CONNECTION);
				}


				}
		}, workerHandle);


	}

	@SuppressLint("MissingPermission")
	public void startClient(){
		// Configuring and starting the subscription session
		SubscribeConfig subscribeConfig = new SubscribeConfig.Builder()
				.setServiceName("serviceName")
				.build();

		ExecutorService queueProcessor = Executors.newSingleThreadExecutor();

		mWifiAwareSession.subscribe(subscribeConfig, new DiscoverySessionCallback() {
			private int messageID = 0;
			private SubscribeDiscoverySession subscribeSession;
			private Queue<Pair<PeerHandle, UUID>> connectionQueue = new LinkedList<>();
			// Current Client Connection
			private Connection clientConnection;
			// Flag to track if the first item has been processed
			boolean isFirstItemProcessed = false;

			// Method to process the next item in the queue
			private void processNextInQueue() {
				if (!connectionQueue.isEmpty()) {
					Pair<PeerHandle, UUID> connectionInfo = connectionQueue.poll();
					Log.d("processNextInQueue", String.format("%s with %s", connectionInfo.first,connectionInfo.second));

					if (nodeType == NodeType.RELAY) {
						// Check if the discovered service's UUID is less than the local UUID
						if (connectionInfo.second.compareTo(myUUID) < 0) {
							subscribeSession.sendMessage(connectionInfo.first, messageID++, FOUND_PEER);
						}
					}
					if (nodeType == NodeType.CLIENT) {
						subscribeSession.sendMessage(connectionInfo.first, messageID++, FOUND_PEER);
					}
				}
			}

			// Submit processing of the next item to the ExecutorService
			private void advanceQueue() {
				queueProcessor.submit(() -> processNextInQueue());
			}

			@Override
			public void onSubscribeStarted(@NonNull SubscribeDiscoverySession session) {
				super.onSubscribeStarted(session);
				Log.d("Subscribe", "Subscriber Started");
				subscribeSession = session;
			}

			@Override
			public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
				super.onServiceDiscovered(peerHandle, serviceSpecificInfo, matchFilter);
				Log.d("onServiceDiscovered", String.format("Discovered: %s", Utils.asUuid(serviceSpecificInfo)));
				connectionQueue.add(new Pair<>(peerHandle,Utils.asUuid(serviceSpecificInfo)));
				// Process the first item immediately when it's discovered
				if (!isFirstItemProcessed) {
					queueProcessor.submit(() -> processNextInQueue());
					isFirstItemProcessed = true;
				}
			}

			@Override
			public void onMessageReceived(PeerHandle peerHandle, byte[] message) {
				super.onMessageReceived(peerHandle, message);

				if (Arrays.equals(message, ACK_PEER)) {
					subscribeSession.sendMessage(peerHandle, messageID++, REQUEST_INTERFACE);
					Log.d("onMessageReceived", String.format("Subscriber: %s from %s", "ACK_PEER",peerHandle.hashCode()));

				}
				if (Arrays.equals(message, ACK_INTERFACE)) {
					subscribeSession.sendMessage(peerHandle, messageID++, REQUEST_CONNECTION);
					Log.d("onMessageReceived", String.format("Subscriber: %s from %s", "ACK_INTERFACE",peerHandle.hashCode()));
				}

				if (Arrays.equals(message, DENY_INTERFACE)) {
					Log.d("onMessageReceived", String.format("Subscriber: %s from %s", "DENY_INTERFACE",peerHandle.hashCode()));
					advanceQueue();
				}

				if (Arrays.equals(message, START_CONNECTION)) {
					clientConnection = new Connection(peerHandle, connectivityManager, subscribeSession);
					clientConnection.startNetworkFromClient();
					connections.add(clientConnection);

					// TODO Error when the exception of network manager not getting the network so clientConnections is wrong, it assumes it as connection and thats why it doesnt dequeue
					Log.d("onMessageReceived", String.format("Subscriber: %s from %s", "START_CONNECTION",peerHandle.hashCode()));

				}
			}

			@Override
			public void onServiceLost(@NonNull PeerHandle peerHandle, int reason) {
				super.onServiceLost(peerHandle, reason);
				Log.d("onServiceLost", String.format("Subscriber: ServiceLost from %s",peerHandle.hashCode()));

				// If lost Peer is the active connection, remove from connections and dequeue next neighbour
				if(clientConnection != null && clientConnection.getPeerHandle().equals(peerHandle)){
					connections.remove(clientConnection);
					clientConnection = null;
					advanceQueue();


				}
			}
		}, workerHandle);

	}


	// Getter and Setters
	public NodeType getNodeType() {
		return nodeType;
	}

	// Pair class for storing two elements together
	class Pair<A, B> {
		private final A first;
		private final B second;

		public Pair(A first, B second) {
			this.first = first;
			this.second = second;
		}

		public A getFirst() {
			return first;
		}

		public B getSecond() {
			return second;
		}
	}

}

