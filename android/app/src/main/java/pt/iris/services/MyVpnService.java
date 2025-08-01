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

import pt.iris.network.lib.WifiAwareLibrary;
import pt.iris.network.lib.TunManager;
import pt.iris.utils.AppContainer;

public class MyVpnService extends VpnService {
   private static final String TAG = "MyVpnService";
   private TunManager tunManager;
   private Thread vpnThread;
   private boolean running = false;
   private final Set<Object> peersWithForwarders = new HashSet<>();

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
	* Simple socket forwarder monitor for TCP connections
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

	   int forwardedCount = 0;
	   for (Map.Entry<?, Socket> entry : peerSockets.entrySet()) {
		   Object peerId = entry.getKey();
		   Socket socket = entry.getValue();

		   if (socket != null && !socket.isClosed() && socket.isConnected()) {
			   try {
				   socket.getOutputStream().write(data, 0, length);
				   socket.getOutputStream().flush();
				   forwardedCount++;
				   Log.v(TAG, "Packet (" + length + " bytes) forwarded to peer: " + peerId);
			   } catch (IOException e) {
				   Log.e(TAG, "Failed to forward to peer " + peerId + ": " + e.getMessage());
			   }
		   } else {
			   Log.v(TAG, "Peer " + peerId + " socket is closed or disconnected");
		   }
	   }

	   Log.d(TAG, "Packet forwarded to " + forwardedCount + "/" + peerSockets.size() + " peers");
   }

   /**
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
												   
												   // Validate packet before writing to TUN
												   if (isValidIPv6Packet(buffer, bytesRead)) {
													   // Forward received data to TUN interface
													   try {
														   tunOut.write(buffer, 0, bytesRead);
														   tunOut.flush();
														   Log.v(TAG, "Successfully wrote " + bytesRead + " bytes to TUN");
													   } catch (IOException writeError) {
														   Log.e(TAG, "Failed to write to TUN interface: " + writeError.getMessage());
														   Log.d(TAG, "Packet details: " + bytesToHex(buffer, Math.min(bytesRead, 64)));
														   // Don't break on write error, continue trying
													   }
													   
													   logPacketDetailed(buffer, bytesRead);
												   } else {
													   Log.w(TAG, "Received invalid IPv6 packet from peer " + peerId + " (" + bytesRead + " bytes), dropping");
													   Log.d(TAG, "Invalid packet details: " + bytesToHex(buffer, Math.min(bytesRead, 64)));
												   }
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
    * Validates if the packet is a proper IPv6 packet
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
	   
	   // Check payload length
	   int payloadLength = ((packet[4] & 0xFF) << 8) | (packet[5] & 0xFF);
	   int expectedLength = payloadLength + 40; // IPv6 header is 40 bytes
	   
	   if (expectedLength != length) {
		   Log.w(TAG, "IPv6 packet length mismatch. Expected: " + expectedLength + ", Actual: " + length);
		   // Don't reject based on this alone, as it might be fragmented or have padding
	   }
	   
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
