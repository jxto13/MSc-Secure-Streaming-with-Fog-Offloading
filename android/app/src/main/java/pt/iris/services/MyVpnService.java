package pt.iris.services;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.content.Intent;
import android.net.VpnService;
import android.os.Build;
import android.os.ParcelFileDescriptor;
import android.util.Log;

import androidx.annotation.RequiresApi;

import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.nio.ByteBuffer;

import pt.iris.network.lib.TunManager;

public class MyVpnService extends VpnService {
	private static final String TAG = "MyVpnService";
	private TunManager tunManager;
	private Thread vpnThread;
	private boolean running = false;

	@Override
	public int onStartCommand(Intent intent, int flags, int startId) {
		Log.i(TAG, "Starting VPN service...");

		// MUST call startForeground() within 5 seconds if started with startForegroundService()
		startForeground(1, createNotification());

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
			try {
				tunManager = new TunManager(getApplicationContext(), this);
				String ulaAddress = tunManager.spinTun();
				Log.i(TAG, "TUN interface established with ULA IPv6: " + ulaAddress);

				RustService.testTUN(ulaAddress);
				ParcelFileDescriptor tun = tunManager.getTunInterface();
				if (tun != null) {
					Log.i(TAG, "TUN fd: " + tun.getFd());

					// Create input/output streams to read/write packets
					FileInputStream in = new FileInputStream(tun.getFileDescriptor());
					FileOutputStream out = new FileOutputStream(tun.getFileDescriptor());

					ByteBuffer packetBuffer = ByteBuffer.allocate(32767); // Max MTU size

					while (running) {
						// Read packet from TUN
						int length = in.read(packetBuffer.array());
						if (length > 0) {
							logPacket(packetBuffer.array(), length); // Log packet contents
						}

						// (Optional) Write packet back or forward as needed
						//out.write(packetBuffer.array(), 0, length);
					}

					in.close();
					out.close();
				}

			} catch (Exception e) {
				Log.e(TAG, "VPN error: ", e);
				stopSelf();
			}
		});
		vpnThread.start();
	}

	/**
	 * Logs the raw packet data in hex format.
	 */
	private void logPacket(byte[] packet, int length) {
		StringBuilder sb = new StringBuilder();
		for (int i = 0; i < length; i++) {
			sb.append(String.format("%02X ", packet[i]));
		}
		Log.d(TAG, "Packet (" + length + " bytes): " + sb.toString());
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
