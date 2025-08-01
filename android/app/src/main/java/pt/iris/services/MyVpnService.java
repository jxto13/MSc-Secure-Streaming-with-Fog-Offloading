package pt.iris.services;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Intent;
import android.net.VpnService;
import android.os.ParcelFileDescriptor;
import android.util.Log;

import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.InetSocketAddress;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;

import pt.iris.network.lib.WifiAwareLibrary;
import pt.iris.network.lib.TunManager;
import pt.iris.utils.AppContainer;

public class MyVpnService extends VpnService {
   private static final String TAG = "MyVpnService";
   private TunManager tunManager;
   private Thread vpnThread;
   private boolean running = false;
   private final Set<Object> peersWithForwarders = new HashSet<>();

   // Routing table: IPv6 address -> peer ID
   private final ConcurrentMap<String, Object> routingTable = new ConcurrentHashMap<>();
   
   // Reference to WifiAwareLibrary (should be set from your app logic)
   private WifiAwareLibrary wifiAwareLibrary;

	@Override
	public int onStartCommand(Intent intent, int flags, int startId) {
		Log.i(TAG, "Starting VPN service...");

		// MUST call startForeground() within 5 seconds if started with startForegroundService()
		startForeground(1, createNotification());

		this.wifiAwareLibrary = AppContainer.getInstance().getNetworkInitializer().getWifiAwareLibrary();

		if (!running) {
			running = true;
			startVpn();
		}

		return START_STICKY;
	}

	private Notification createNotification() {
		NotificationChannel channel = new NotificationChannel(
				"vpn_channel",
				"VPN Service",
				NotificationManager.IMPORTANCE_LOW
		);
		NotificationManager manager = getSystemService(NotificationManager.class);
		if (manager != null) {
			manager.createNotificationChannel(channel);
		}

		return new Notification.Builder(this, "vpn_channel")
				.setContentTitle("VPN Running")
				.setContentText("Your VPN is active")
				.setSmallIcon(android.R.drawable.ic_secure) // Use your VPN icon
				.build();
	}

	private void startVpn() {
		vpnThread = new Thread(() -> {
			Log.i(TAG, "VPN thread started");
			try {
				tunManager = new TunManager(getApplicationContext(), this);
				String ulaAddress = tunManager.spinTun();
				Log.i(TAG, "TUN interface established with ULA IPv6: " + ulaAddress);

				// Set the TUN IP in WifiAwareLibrary 
				if (wifiAwareLibrary != null) {
					wifiAwareLibrary.setLocalTunIP(ulaAddress);
				}

				RustService.testTUN(ulaAddress);

				ParcelFileDescriptor tun = tunManager.getTunInterface();
				if (tun != null) {
					Log.i(TAG, "TUN file descriptor obtained: " + tun.getFd());

					FileInputStream tunIn = new FileInputStream(tun.getFileDescriptor());
					FileOutputStream tunOut = new FileOutputStream(tun.getFileDescriptor());

					// Start TCP-to-TUN forwarding threads
					Log.i(TAG, "Starting TCP-to-TUN forwarder threads...");
					startSocketForwarderMonitor(tunOut);

					// Start TUN-to-TCP forwarding loop
					Log.i(TAG, "Starting TUN-to-TCP forwarding loop...");
					byte[] buffer = new byte[32767]; // Max MTU size

						while (running) {
							int length = tunIn.read(buffer);
							if (length > 0) {
								logPacketDetailed(buffer, length);
								forwardPacketToAllPeers(buffer, length);
							}
						}					Log.i(TAG, "TUN forwarding loop exited, closing streams...");
					tunIn.close();
					tunOut.close();
				} else {
					Log.e(TAG, "Failed to get TUN interface (tun=null)");
				}

			} catch (IOException e) {
				Log.e(TAG, "VPN I/O error: " + e.getMessage(), e);
			} catch (Exception e) {
				Log.e(TAG, "Unexpected VPN error: ", e);
			} finally {
				Log.i(TAG, "VPN thread stopping, cleaning up...");
				stopSelf();
			}
		}, "VPN-Thread");
		vpnThread.start();
	}

	private void logPacketDetailed(byte[] packet, int length) {
		if (length < 40) { // Minimum IPv6 header size
			Log.w(TAG, "Packet too short (" + length + " bytes), skipping log");
			return;
		}

		// Extract IPv6 header fields
		String srcAddr = formatIpv6Address(packet, 8);   // Bytes 8-23
		String dstAddr = formatIpv6Address(packet, 24);  // Bytes 24-39
		int nextHeader = packet[6] & 0xFF;               // Protocol
		String protoName = (nextHeader == 6) ? "TCP" :
				(nextHeader == 17) ? "UDP" :
						(nextHeader == 58) ? "ICMPv6" : "Other(" + nextHeader + ")";

		Log.d(TAG, String.format("IPv6 Packet: %d bytes | %s -> %s | Proto=%s",
				length, srcAddr, dstAddr, protoName));
	}

	private String formatIpv6Address(byte[] packet, int offset) {
		StringBuilder sb = new StringBuilder();
		for (int i = 0; i < 16; i += 2) {
			sb.append(String.format("%02x%02x", packet[offset + i], packet[offset + i + 1]));
			if (i < 14) sb.append(":");
		}
		return sb.toString();
	}

	/**
	 * Learn routing: associate an IPv6 source address with a peer ID
	 * Only updates if route is new or changed to avoid unnecessary overhead
	 */
	private void learnRoute(String ipv6Address, Object peerId) {
		if (ipv6Address != null && peerId != null) {
			Object existingPeer = routingTable.get(ipv6Address);
			if (!peerId.equals(existingPeer)) {
				routingTable.put(ipv6Address, peerId);
				if (existingPeer == null) {
					Log.v(TAG, "Learned new route: " + ipv6Address + " -> peer " + peerId);
				} else {
					Log.v(TAG, "Updated route: " + ipv6Address + " -> peer " + peerId + " (was: " + existingPeer + ")");
				}
			}
			// No log if route already exists and hasn't changed
		}
	}

	/**
	 * Look up which peer to route to for a given IPv6 destination
	 */
	private Object lookupRoute(String ipv6Address) {
		return routingTable.get(ipv6Address);
	}

	/**
	 * Extract IPv6 destination address from packet
	 */
	private String extractDestinationIPv6(byte[] packet, int length) {
		if (length < 40) return null; // Not a valid IPv6 packet
		return formatIpv6Address(packet, 24); // Destination is at offset 24-39
	}

	/**
	 * Extract IPv6 source address from packet
	 */
	private String extractSourceIPv6(byte[] packet, int length) {
		if (length < 40) return null; // Not a valid IPv6 packet
		return formatIpv6Address(packet, 8); // Source is at offset 8-23
	}



   /**
    * Forward packet from TUN to appropriate peer(s) using routing table
    */
   private void forwardPacketToAllPeers(byte[] data, int length) {
	   if (wifiAwareLibrary == null) {
		   Log.w(TAG, "forwardPacketToAllPeers: WifiAwareLibrary is null, cannot forward.");
		   return;
	   }

	   Map<?, Socket> peerSockets = wifiAwareLibrary.getPeerClientSockets();
	   
	   if (peerSockets.isEmpty()) {
		   Log.v(TAG, "No peer TCP sockets available for forwarding");
		   return;
	   }

	   // Extract destination IPv6 address from packet
	   String destIPv6 = extractDestinationIPv6(data, length);
	   if (destIPv6 == null) {
		   Log.w(TAG, "Cannot extract destination IPv6 from packet, dropping");
		   return;
	   }

	   // Look up the specific peer for this destination
	   Object targetPeerId = lookupRoute(destIPv6);
	   
	   if (targetPeerId != null) {
		   // Route to specific peer
		   Socket targetSocket = peerSockets.get(targetPeerId);
		   if (targetSocket != null && !targetSocket.isClosed() && targetSocket.isConnected()) {
			   try {
				   targetSocket.getOutputStream().write(data, 0, length);
				   targetSocket.getOutputStream().flush();
				   Log.v(TAG, "Packet (" + length + " bytes) routed to specific peer: " + targetPeerId + " for dest: " + destIPv6);
				   return; // Successfully routed
			   } catch (IOException e) {
				   Log.e(TAG, "Failed to route to peer " + targetPeerId + ": " + e.getMessage());
				   // Remove invalid route and fall back to broadcast
				   routingTable.remove(destIPv6);
			   }
		   } else {
			   Log.w(TAG, "Target peer " + targetPeerId + " socket is closed, removing route");
			   routingTable.remove(destIPv6);
		   }
	   }

	   // No specific route found or routing failed, broadcast to all peers
	   Log.v(TAG, "No specific route for " + destIPv6 + ", broadcasting to all peers");
	   int forwardedCount = 0;
	   for (Map.Entry<?, Socket> entry : peerSockets.entrySet()) {
		   Object peerId = entry.getKey();
		   Socket socket = entry.getValue();

		   if (socket != null && !socket.isClosed() && socket.isConnected()) {
			   try {
				   socket.getOutputStream().write(data, 0, length);
				   socket.getOutputStream().flush();
				   forwardedCount++;
				   Log.v(TAG, "Packet (" + length + " bytes) broadcasted to peer: " + peerId);
			   } catch (IOException e) {
				   Log.e(TAG, "Failed to broadcast to peer " + peerId + ": " + e.getMessage());
			   }
		   } else {
			   Log.v(TAG, "Peer " + peerId + " socket is closed or disconnected");
		   }
	   }

	   Log.d(TAG, "Packet broadcasted to " + forwardedCount + "/" + peerSockets.size() + " peers for dest: " + destIPv6);
   }   /**
    * Socket forwarder that reads from TCP connections and forwards to TUN interface
    */
   private void startSocketForwarderMonitor(FileOutputStream tunOut) {
	   if (wifiAwareLibrary == null) {
		   Log.w(TAG, "startSocketForwarderMonitor: WifiAwareLibrary is null, cannot start monitor.");
		   return;
	   }

	   new Thread(() -> {
		   Log.d(TAG, "Socket forwarder monitor started for TCP connections");
		   
		   while (running) {
			   try {
				   // Get both client and server sockets for bidirectional communication
				   Map<?, Socket> clientSockets = wifiAwareLibrary.getPeerClientSockets();
				   Map<?, ServerSocket> serverSockets = wifiAwareLibrary.getPeerServerSockets();

				   Log.v(TAG, "Monitoring " + clientSockets.size() + " client sockets");
				   
				   synchronized (peersWithForwarders) {
					   // Remove peers whose sockets are closed
					   peersWithForwarders.removeIf(peerId -> {
						   Socket s = clientSockets.get(peerId);
						   return s == null || s.isClosed() || !s.isConnected();
					   });

					   // Start forwarders for new client socket connections
					   for (Map.Entry<?, Socket> entry : clientSockets.entrySet()) {
						   Object peerId = entry.getKey();
						   Socket socket = entry.getValue();
						   
						   if (socket != null && !peersWithForwarders.contains(peerId) && 
							   !socket.isClosed() && socket.isConnected()) {
							   
							   peersWithForwarders.add(peerId);
							   Log.i(TAG, "Launching TCP→TUN forwarder thread for peer: " + peerId);
							   
							   new Thread(() -> {
								   try {
									   InputStream inputStream = socket.getInputStream();
									   byte[] buffer = new byte[32768]; // 32KB buffer
									   
									   while (running && !socket.isClosed() && socket.isConnected()) {
										   try {
											   int bytesRead = inputStream.read(buffer);
											   if (bytesRead > 0) {
												   // Check if this is a keepalive ping message (text-based check)
												   boolean isPing = false;
												   if (bytesRead >= 13) { // "PING_KEEPALIVE" is 13 chars + newline
													   try {
														   String potentialPing = new String(buffer, 0, Math.min(bytesRead, 50), "UTF-8");
														   if (potentialPing.startsWith("PING_KEEPALIVE")) {
															   isPing = true;
															   Log.v(TAG, "Received keepalive ping from peer " + peerId + ", ignoring");
														   }
													   } catch (Exception e) {
														   // Not a text message, continue as normal packet
													   }
												   }
												   
												   if (isPing) {
													   continue; // Skip ping messages
												   }
												   
												   Log.v(TAG, "Received " + bytesRead + " bytes from peer " + peerId + ", forwarding to TUN");
												   
												   // Learn the route: map source IPv6 to this peer (if valid IPv6)
												   String sourceIPv6 = extractSourceIPv6(buffer, bytesRead);
												   if (sourceIPv6 != null) {
													   learnRoute(sourceIPv6, peerId);
												   }
												   
												   // Forward received data to TUN interface (no validation, forward everything except pings)
												   try {
													   tunOut.write(buffer, 0, bytesRead);
													   tunOut.flush();
													   Log.v(TAG, "Successfully wrote " + bytesRead + " bytes to TUN from peer " + peerId);
												   } catch (IOException writeError) {
													   Log.e(TAG, "Failed to write to TUN interface: " + writeError.getMessage());
													   Log.d(TAG, "Packet details: " + bytesToHex(buffer, Math.min(bytesRead, 64)));
												   }
												   
												   logPacketDetailed(buffer, bytesRead);
											   } else if (bytesRead == -1) {
												   Log.d(TAG, "End of stream reached for peer: " + peerId);
												   break;
											   }
										   } catch (IOException e) {
											   if (running && !socket.isClosed()) {
												   Log.w(TAG, "TCP read error for peer " + peerId + ": " + e.getMessage());
											   }
											   break;
										   }
									   }
									   
									   Log.i(TAG, "TCP→TUN forwarder thread for peer " + peerId + " exiting");
								   } catch (Exception e) {
									   Log.w(TAG, "TCP→TUN forwarder error for peer " + peerId + ": " + e.getMessage());
								   } finally {
									   synchronized (peersWithForwarders) {
										   peersWithForwarders.remove(peerId);
									   }
								   }
							   }, "TcpToTun-" + peerId).start();
						   }
					   }
				   }
				   
				   Thread.sleep(2000); // Check for new connections every 2 seconds
				   
			   } catch (InterruptedException e) {
				   Log.d(TAG, "Socket forwarder monitor interrupted");
				   break;
			   } catch (Exception e) {
				   Log.e(TAG, "Error in socket forwarder monitor", e);
			   }
		   }
		   
		   	   Log.d(TAG, "Socket forwarder monitor exiting");
	   }, "SocketForwarderMonitor").start();
   }



   /**
    * Simplified packet validation - only check if it's potentially IPv6
    * No strict length validation to avoid dropping valid packets
    */
   private boolean isValidIPv6Packet(byte[] packet, int length) {
	   if (length < 40) { // Minimum IPv6 header size
		   Log.w(TAG, "Packet too short for IPv6: " + length + " bytes");
		   return false;
	   }
	   
	   // Check IPv6 version (first 4 bits should be 6)
	   int version = (packet[0] & 0xF0) >> 4;
	   if (version != 6) {
		   Log.w(TAG, "Not an IPv6 packet, version: " + version);
		   return false;
	   }
	   
	   // Remove strict length validation that was causing issues
	   // IPv6 packets can have variable lengths due to padding, fragmentation, etc.
	   
	   return true;
   }

   /**
    * Converts bytes to hex string for debugging
    */
   private String bytesToHex(byte[] bytes, int length) {
	   StringBuilder sb = new StringBuilder();
	   for (int i = 0; i < length && i < bytes.length; i++) {
		   sb.append(String.format("%02x", bytes[i]));
		   if (i % 16 == 15) sb.append(" ");
		   else if (i % 2 == 1) sb.append(" ");
	   }
	   return sb.toString();
   }


   @Override
   public void onDestroy() {
	   Log.i(TAG, "Stopping VPN service...");
	   running = false;
	   
	   if (tunManager != null) {
		   tunManager.closeTun();
	   }
	   if (vpnThread != null) {
		   vpnThread.interrupt();
	   }
	   super.onDestroy();
   }

   @Override
   public android.os.IBinder onBind(Intent intent) {
	   // Required for VpnService binding
	   return super.onBind(intent);
   }

}
