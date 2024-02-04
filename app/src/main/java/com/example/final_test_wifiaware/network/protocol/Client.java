package com.example.final_test_wifiaware.network.protocol;

import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.WifiAwareNetworkInfo;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.os.Build;
import android.util.Log;

import androidx.annotation.NonNull;
import androidx.annotation.RequiresApi;

import com.example.final_test_wifiaware.utils.AppContainer;

import java.io.IOException;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.SocketChannel;
import java.nio.charset.StandardCharsets;

/**
 * Represents a client that connects to a server using Wi-Fi Aware technology.
 */
public class Client implements Runnable {

    private static final String TAG = "Client";
    private static final int BUFFER_SIZE = 8192;

    // Connectivity Manager for managing network requests
    private final ConnectivityManager mConnManager;

    // Current network and its capabilities
    private Network mCurrentNet;
    private NetworkCapabilities mCurrentNetCapabilities;

    // Socket channel for communication
    private SocketChannel mSocketChannel_1;

    // Write buffer for sending data
    private ByteBuffer mWriteBuffer = ByteBuffer.allocate(BUFFER_SIZE);

    // Thread for running the client
    private Thread mClientThread;

    // Flag to track client status
    private boolean mEnabled = false;

    /**
     * Constructor for the Client class.
     *
     * @param manager           The ConnectivityManager instance for managing network requests.
     * @param subscribeSession The Wi-Fi Aware discovery session.
     * @param handle            The PeerHandle representing the server peer.
     */
    public Client(ConnectivityManager manager, DiscoverySession subscribeSession, PeerHandle handle) {
        mCurrentNetCapabilities = null;
        mCurrentNet = null;
        mSocketChannel_1 = null;
        mConnManager = manager;

        // Build network specifier for the connection
        WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(subscribeSession, handle)
                .build();

        // Build network request for the connection
        NetworkRequest networkRequest = new NetworkRequest.Builder()
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
                .setNetworkSpecifier(networkSpecifier)
                .build();

        // Request a network with the specified network request and register network callback
        mConnManager.requestNetwork(networkRequest, new WifiAwareNetworkCallback());
    }

    /**
     * Start the client by initializing the client thread.
     */
    public void start() {
        if (!mEnabled && mCurrentNetCapabilities != null) {
            mEnabled = true;
            mClientThread = new Thread(this);
            mClientThread.start();
        }
    }

    /**
     * Stop the client by interrupting the client thread.
     */
    public void stop() {
        if (mEnabled) {
            mEnabled = false;
            mClientThread.interrupt();
        }
    }

    // Run method for the client thread
    @Override
    public void run() {
        try {
            // Bind the process to the current network
            //if (!mConnManager.bindProcessToNetwork(mCurrentNet)) return;

            // Extract information from the network capabilities
            WifiAwareNetworkInfo peerAwareInfo = (WifiAwareNetworkInfo) mCurrentNetCapabilities.getTransportInfo();
            InetAddress peerIpv6 = peerAwareInfo.getPeerIpv6Addr();
            int peerPort = peerAwareInfo.getPort();

            // Open a socket channel and connect to the server
            mSocketChannel_1 = SocketChannel.open();

            mSocketChannel_1.connect(new InetSocketAddress(peerIpv6, peerPort));

            while (mEnabled) {
                // Prepare data to be sent
                mWriteBuffer.clear();
                mWriteBuffer.put(new String("Hello from " + AppContainer.getInstance().getMyUUID().toString()).getBytes(StandardCharsets.UTF_8));
                mWriteBuffer.flip();

                // Write data to the socket channel
                mSocketChannel_1.write(mWriteBuffer);

                // Sleep
                Thread.sleep(50000);
            }

            // Close the socket channel
            mSocketChannel_1.close();
        } catch (IOException | InterruptedException e) {
            Log.d(TAG, "Client exception: " + e.toString());
        } finally {
            // Clean up resources
            mEnabled = false;
            mClientThread = null;
            mSocketChannel_1 = null;
            mCurrentNetCapabilities = null;
            mCurrentNet = null;
        }
    }

    /**
     * Represents a network callback for Wi-Fi Aware.
     */
    private class WifiAwareNetworkCallback extends ConnectivityManager.NetworkCallback {

        @Override
        public void onAvailable(@NonNull Network network) {
            // Set the current network when available
            mCurrentNet = network;
        }

        @Override
        public void onCapabilitiesChanged(@NonNull Network network, @NonNull NetworkCapabilities networkCapabilities) {
            if (mCurrentNetCapabilities == null) {
                // Set network capabilities and start the client if not already enabled
                mCurrentNetCapabilities = networkCapabilities;
                if (!mEnabled) start();
            } else {
                Log.d(TAG, "onCapabilitiesChanged: Different network or new capability");
            }
        }

        @Override
        public void onLost(@NonNull Network network) {
            // Unregister network callback and stop the client
            mConnManager.unregisterNetworkCallback(this);
            stop();
        }

        @Override
        public void onUnavailable() {
            Log.d(TAG,"onUnavailable");
        }
    }
}
