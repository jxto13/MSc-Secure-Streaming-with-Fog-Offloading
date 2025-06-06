package pt.iris.services;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;

import java.nio.ByteBuffer;
import android.net.wifi.WifiManager;
import android.content.Context;

public class RustService extends Service {

    private WifiManager.MulticastLock multicastLock;

    static {
        System.loadLibrary("irislinker");
    }

    private static native void startService();
    private static native void stopService();
    public static native void startClient(int fd);

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
        startService();
        return START_STICKY; // consider using STICKY for resilience
    }

    @Override
    public void onDestroy() {
        super.onDestroy();

        // Stop native service
        stopService();

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
