package pt.iris.services;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;

import android.net.wifi.WifiManager;
import android.content.Context;

public class RustService extends Service {

    private WifiManager.MulticastLock multicastLock;

    static {
        System.loadLibrary("irislinker");
    }

    public static native void startClient(int fd, String streamUrl);
    public static native void addSocket(String ipAddress);
    public static native void removeSocket(String ipAddress);
    public static native void cleanupAllSockets(); // Add cleanup method
    
    // Network statistics methods
    public static native double getPacketLossPercentage();
    public static native double getCurrentBitrateMbps();
    public static native long getTotalDataTransferredMB();
    public static native int getActiveConnectionCount();

    @Override
    public void onCreate() {
        super.onCreate();

        // Acquire the multicast lock
        WifiManager wifi = (WifiManager) getApplicationContext().getSystemService(Context.WIFI_SERVICE);
        multicastLock = wifi.createMulticastLock("RustServiceMulticastLock");
        multicastLock.setReferenceCounted(true);
        multicastLock.acquire();
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        super.onStartCommand(intent, flags, startId);
        // Service startup logic can be added here if needed
        return START_STICKY; // consider using STICKY for resilience
    }

    @Override
    public void onDestroy() {
        super.onDestroy();

        // Cleanup all active connections
        try {
            cleanupAllSockets();
        } catch (UnsatisfiedLinkError e) {
            // Native library might not be loaded, ignore
        }

        // Release multicast lock
        if (multicastLock != null && multicastLock.isHeld()) {
            multicastLock.release();
        }
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }
}
