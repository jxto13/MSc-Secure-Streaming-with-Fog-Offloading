package com.example.final_test_wifiaware.services;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;

import java.io.FileDescriptor;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;

public class RustService extends Service {

    // Load the native library when the class is loaded
    static {
        System.loadLibrary("rustapp");
    }

    /**
     * Native method declaration to start the Rust service.
     */
    private static native void startService();
    public static native void writeToDirectBuffer(ByteBuffer buffer);
    public static native byte[] helloByte(int size);
    public static native void tmpFile();
    public static native void fdTest(String input);

    @Override
    public void onCreate() {
        super.onCreate();
        // Service initialization logic can be added here
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        super.onStartCommand(intent, flags, startId);

        // Start the Rust service with the path to the files directory
        startService();

        // Indicate that the service should be restarted if it gets terminated
        return START_STICKY;
    }

    @Override
    public void onDestroy() {
        super.onDestroy();
        // Cleanup or resource release logic can be added here
    }

    @Override
    public IBinder onBind(Intent intent) {
        // Return null as this service does not support binding
        return null;
    }

}
