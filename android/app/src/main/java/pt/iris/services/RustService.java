package pt.iris.services;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;

import java.nio.ByteBuffer;

public class RustService extends Service {

    // Load the native library when the class is loaded
    static {
        System.loadLibrary("irislinker");
    }

    /**
     * Native method declaration to start the Rust service.
     */
    private static native void startService();
    private static native void stopService();

    private static native void startClient(String hostname);

    @Override
    public void onCreate() {
        super.onCreate();
        // Service initialization logic can be added here
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        super.onStartCommand(intent, flags, startId);

        // Start the Rust service
        startService();

        // Indicate that the service should NOT be restarted if it gets terminated
        return START_NOT_STICKY;
    }

    @Override
    public void onDestroy() {
        super.onDestroy();
        // Cleanup or resource release logic can be added here

        // Stop the Rust service
        stopService();
    }

    @Override
    public IBinder onBind(Intent intent) {
        // Return null as this service does not support binding
        return null;
    }

}
