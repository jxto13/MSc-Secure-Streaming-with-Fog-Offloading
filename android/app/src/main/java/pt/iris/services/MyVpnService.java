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
	private FileOutputStream tunOut; // Store TUN output stream for callbacks
   
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
					// Register callback for immediate socket notifications
					wifiAwareLibrary.setSocketCallback(new WifiAwareLibrary.SocketCallback() {
						@Override
						public void onNewSocketAvailable(Object peerId, Socket socket) {
							Log.i(TAG, "New socket available for peer: " + peerId);
							startSocketForwarder(peerId, socket, tunOut);
						}
						
						@Override
						public void onSocketClosed(Object peerId) {
							Log.i(TAG, "Socket closed for peer: " + peerId);
							synchronized (peersWithForwarders) {
								peersWithForwarders.remove(peerId);
							}
						}
					});
				}

				ParcelFileDescriptor tun = tunManager.getTunInterface();
				if (tun != null) {
					Log.i(TAG, "TUN file descriptor obtained: " + tun.getFd());

					FileInputStream tunIn = new FileInputStream(tun.getFileDescriptor());
					tunOut = new FileOutputStream(tun.getFileDescriptor());

					// Start initial socket forwarders for existing connections
					Log.i(TAG, "Starting initial socket forwarders...");
					startInitialSocketForwarders(tunOut);

					// Start TUN-to-TCP forwarding loop
					Log.i(TAG, "Starting TUN-to-TCP forwarding loop...");
					byte[] buffer = new byte[32767]; // Max MTU size

					while (running) {
						int length = tunIn.read(buffer);
						if (length > 0) {
							Log.v(TAG, "Read " + length + " bytes from TUN, forwarding to all peers");
							forwardPacketToAllPeers(buffer, length);
						}
					}
					Log.i(TAG, "TUN forwarding loop exited, closing streams...");
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
		Log.v(TAG, "Packet: " + length + " bytes");
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
    * Forward packet from TUN to all connected peer sockets (simplified)
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

	   // Simply forward to all connected peers
	   int forwardedCount = 0;
	   for (Map.Entry<?, Socket> entry : peerSockets.entrySet()) {
		   Object peerId = entry.getKey();
		   Socket socket = entry.getValue();

		   if (socket != null && !socket.isClosed() && socket.isConnected()) {
			   try {
				   socket.getOutputStream().write(data, 0, length);
				   socket.getOutputStream().flush();
				   forwardedCount++;
				   Log.v(TAG, "Packet (" + length + " bytes) sent to peer: " + peerId);
			   } catch (IOException e) {
				   Log.e(TAG, "Failed to send to peer " + peerId + ": " + e.getMessage());
			   }
		   } else {
			   Log.v(TAG, "Peer " + peerId + " socket is closed or disconnected");
		   }
	   }

	   Log.v(TAG, "Packet sent to " + forwardedCount + "/" + peerSockets.size() + " peers");
   }   /**
    * Start initial socket forwarders for any existing connections
    */
   private void startInitialSocketForwarders(FileOutputStream tunOut) {
	   if (wifiAwareLibrary == null) {
		   Log.w(TAG, "startInitialSocketForwarders: WifiAwareLibrary is null, cannot start forwarders.");
		   return;
	   }

	   Map<?, Socket> clientSockets = wifiAwareLibrary.getPeerClientSockets();
	   Log.d(TAG, "Starting initial forwarders for " + clientSockets.size() + " existing client sockets");
	   
	   for (Map.Entry<?, Socket> entry : clientSockets.entrySet()) {
		   Object peerId = entry.getKey();
		   Socket socket = entry.getValue();
		   
		   if (socket != null && !socket.isClosed() && socket.isConnected()) {
			   startSocketForwarder(peerId, socket, tunOut);
		   }
	   }
   }

   /**
    * Start a socket forwarder for a specific peer (called via callback)
    */
   private void startSocketForwarder(Object peerId, Socket socket, FileOutputStream tunOut) {
	   synchronized (peersWithForwarders) {
		   if (peersWithForwarders.contains(peerId)) {
			   Log.d(TAG, "Forwarder already exists for peer: " + peerId);
			   return;
		   }
		   peersWithForwarders.add(peerId);
	   }
	   
	   Log.i(TAG, "Starting TCP→TUN forwarder thread for peer: " + peerId);
	   
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

						   // Simply forward all non-ping data to TUN interface
						   try {
							   tunOut.write(buffer, 0, bytesRead);
							   tunOut.flush();
							   Log.v(TAG, "Successfully wrote " + bytesRead + " bytes to TUN from peer " + peerId);
						   } catch (IOException writeError) {
							   Log.e(TAG, "Failed to write to TUN interface: " + writeError.getMessage());
							   continue;
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
