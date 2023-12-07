package com.example.final_test_wifiaware.network.protocol;

import android.annotation.SuppressLint;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.net.NetworkSpecifier;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.WifiAwareNetworkSpecifier;
import android.util.Log;

import androidx.annotation.NonNull;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.ByteChannel;
import java.nio.channels.SelectableChannel;
import java.nio.channels.SelectionKey;
import java.nio.channels.Selector;
import java.nio.channels.ServerSocketChannel;
import java.nio.channels.SocketChannel;
import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.locks.Lock;
import java.util.concurrent.locks.ReentrantLock;

/**
 * Represents a server that manages connections using Wi-Fi Aware technology.
 */
public class Server implements Runnable {
    private static final String TAG = "Server";

    private static final int BUFFER_SIZE = 8192;

    // Selector for managing channels
    private final Selector mSelector;

    // Thread for running the server
    private Thread mServerThread;

    // Flag to track server status
    private boolean mEnabled = false;

    // Buffer for reading data
    private final ByteBuffer mReadBuffer = ByteBuffer.allocate(BUFFER_SIZE);

    // Maps to keep track of connections
    private Map<PeerHandle, Connection> mConnectionsMap;
    private Map<ServerSocketChannel, Connection> mServerChannelsMap;
    private Map<UUID, Connection> uuidConnectionMap;

    // Android ConnectivityManager for managing network requests
    private ConnectivityManager mConManager;

    // Lock for ensuring thread safety in selector operations
    private final Lock mSelectorLock = new ReentrantLock();

    // Channels for writing to TextViews
    private ByteChannel mCanalTextView1 = null;
    private ByteChannel mCanalTextView2 = null;

    /**
     * Constructor for the Server class.
     *
     * @param manager The ConnectivityManager instance for managing network requests.
     * @throws IOException If an I/O error occurs while opening the selector.
     */
    @SuppressLint("NewApi")
    public Server(ConnectivityManager manager) throws IOException {
        mSelector = Selector.open();
        mConnectionsMap = new HashMap<>();
        mServerChannelsMap = new HashMap<>();
        mConManager = manager;
    }

    /**
     * Starts the server by initializing the server thread.
     */
    public void start() {
        if (!mEnabled) {
            mEnabled = true;
            mServerThread = new Thread(this);
            mServerThread.start();
        }
    }

    /**
     * Stops the server by interrupting the server thread and closing all connections.
     */
    public void stop() {
        if (mEnabled) {
            mEnabled = false;
            mServerThread.interrupt();
            closeAllConnections();
            try {
                mSelector.close();
            } catch (IOException e) {
                // Handle exception
            }
        }
    }

    // Run method for the server thread
    @Override
    public void run() {
        mSelectorLock.lock();
        while (mEnabled) {
            try {
                mSelectorLock.unlock();
                mSelector.select();
                mSelectorLock.lock();
                Iterator<SelectionKey> itKeys = mSelector.selectedKeys().iterator();
                while (itKeys.hasNext()) {
                    SelectionKey myKey = itKeys.next();
                    itKeys.remove();

                    if (!myKey.isValid()) {
                        continue;
                    }

                    if (myKey.isAcceptable()) {
                        accept(myKey);
                    } else if (myKey.isReadable()) {
                        this.read(myKey);
                    }
                }
            } catch (IOException e) {
                mEnabled = false;
            }
        }
        mSelectorLock.unlock();
    }

    /**
     * Add a new connection.
     *
     * @param discoverySession The Wi-Fi Aware discovery session.
     * @param handle            The PeerHandle representing the peer.
     * @return True if the connection is added successfully; false otherwise.
     */
    @SuppressLint("NewApi")
    public synchronized boolean addNewConnection(DiscoverySession discoverySession, PeerHandle handle) {
        if (mConnectionsMap.get(handle) != null) {
            return true; // Connection already exists
        }
        if (!mEnabled) return false; // Server not enabled

        ServerSocketChannel serverSocketChannel = null;
        int mServerPort;
        try {
            // Create a new server socket channel
            serverSocketChannel = ServerSocketChannel.open();
            serverSocketChannel.configureBlocking(false);
            serverSocketChannel.socket().bind(new InetSocketAddress(0));
            mServerPort = serverSocketChannel.socket().getLocalPort();

            // Build network specifier for the connection
            WifiAwareNetworkSpecifier networkSpecifier = new WifiAwareNetworkSpecifier.Builder(discoverySession, handle)
                    .setPskPassphrase("wifiawaretest")
                    .setPort(mServerPort)
                    .setTransportProtocol(17)
                    .build();

            // Build network request for the connection
            NetworkRequest networkRequest = new NetworkRequest.Builder()
                    .addTransportType(NetworkCapabilities.TRANSPORT_WIFI_AWARE)
                    .setNetworkSpecifier(networkSpecifier)
                    .build();

            mSelectorLock.lock();
            mSelector.wakeup();
            try {
                // Register the server socket channel with the selector for accepting connections
                SelectionKey serverKey = serverSocketChannel.register(mSelector, SelectionKey.OP_ACCEPT);
            } catch (IOException ex) {
                mSelectorLock.unlock();
                serverSocketChannel.close();
                throw ex;
            }
            mSelectorLock.unlock();

            // Create a new Connection object and add it to the maps
            Connection conn = new Connection(serverSocketChannel, handle);
            mConnectionsMap.put(handle, conn);
            mServerChannelsMap.put(serverSocketChannel, conn);

            // Request a network with the specified network request
            mConManager.requestNetwork(networkRequest, new WifiAwareNetworkCallback(handle));
        } catch (IOException e) {
            return false; // Unable to create connection
        }
        return true;
    }

    /**
     * Remove a connection.
     *
     * @param handle The PeerHandle representing the peer.
     */
    public synchronized void removeConnection(PeerHandle handle) {
        Connection conn = mConnectionsMap.get(handle);
        ServerSocketChannel serverChan = null;
        if (conn == null) {
            return; // Connection not found
        }
        serverChan = conn.mServerSocketChannel;
        mSelectorLock.lock();
        mSelector.wakeup();
        conn.closeConnection(mSelector);
        mSelectorLock.unlock();
        mConnectionsMap.remove(handle);
        mServerChannelsMap.remove(serverChan);
    }

    /**
     * Close all connections.
     */
    private synchronized void closeAllConnections() {
        mSelectorLock.lock();
        for (Connection conn : mConnectionsMap.values()) {
            conn.closeConnection(mSelector);
        }
        mConnectionsMap.clear();
        mServerChannelsMap.clear();
        mSelectorLock.unlock();
    }

    /**
     * Accept an incoming connection.
     *
     * @param key The SelectionKey associated with the server socket channel.
     */
    private synchronized void accept(@NonNull SelectionKey key) {
        ServerSocketChannel serverChan = (ServerSocketChannel) key.channel();
        Connection conn = mServerChannelsMap.get(serverChan);
        if (conn == null) {
            return; // Connection not found
        }
        SocketChannel socketChannel = null;
        try {
            // Accept the connection and make it non-blocking
            socketChannel = serverChan.accept();
            socketChannel.configureBlocking(false);
            // Register the socket channel with the selector for reading
            socketChannel.register(mSelector, SelectionKey.OP_READ);
            // Add the socket channel to the list of communication channels
            conn.mComChannels.add(socketChannel);
        } catch (IOException e) {
            // Handle exception
            mConnectionsMap.remove(conn.handle);
            mServerChannelsMap.remove(serverChan);
            conn.closeConnection(mSelector);
        }
    }

    /**
     * Read data from a channel.
     *
     * @param key The SelectionKey associated with the socket channel.
     */
    private void read(@NonNull SelectionKey key) {
        int numRead;
        SelectableChannel socketChannel = key.channel();
        mReadBuffer.clear();
        try {
            ByteChannel bChan = (ByteChannel) socketChannel;
            if (mCanalTextView1 == null) {
                mCanalTextView1 = bChan;
            } else if (mCanalTextView2 == null) mCanalTextView2 = bChan;
            numRead = bChan.read(mReadBuffer);
            if (numRead != -1) {
                byte[] bytes = mReadBuffer.array();
                String v = new String(bytes, 0, numRead, StandardCharsets.UTF_8);
                Log.d(TAG, "read: " + v);
            }
        } catch (IOException e) {
            socketChannel.keyFor(mSelector).cancel();
        }
    }

    /**
     * Convert ByteBuffer to String.
     *
     * @param buffer  The ByteBuffer to convert.
     * @param charset The Charset to use for conversion.
     * @return The converted String.
     */
    public static String bb_to_str(ByteBuffer buffer, Charset charset) {
        byte[] bytes;
        if (buffer.hasArray()) {
            bytes = buffer.array();
        } else {
            bytes = new byte[buffer.remaining()];
            buffer.get(bytes);
        }
        return new String(bytes, charset);
    }

    /**
     * Represents a connection with a peer.
     */
    private class Connection {
        public Connection(ServerSocketChannel serverChan, PeerHandle handle) {
            mServerSocketChannel = serverChan;
            this.handle = handle;
            mComChannels = new ArrayList<>();
        }

        public ServerSocketChannel mServerSocketChannel;
        public PeerHandle handle;
        public Network net;
        public List<SocketChannel> mComChannels;

        /**
         * Close the connection.
         *
         * @param selector The Selector for managing channels.
         */
        public void closeConnection(Selector selector) {
            try {
                SelectionKey selKey = this.mServerSocketChannel.keyFor(selector);
                if (selKey != null) selKey.cancel();
                this.mServerSocketChannel.close();
                for (SocketChannel chan : this.mComChannels) {
                    selKey = chan.keyFor(selector);
                    if (selKey != null) selKey.cancel();
                    chan.close();
                }
            } catch (IOException e) {
                // Handle exception
            } finally {
                this.mServerSocketChannel = null;
                this.handle = null;
                this.net = null;
                this.mComChannels.clear();
            }
        }
    }

    /**
     * Represents a network callback for Wi-Fi Aware.
     */
    private class WifiAwareNetworkCallback extends ConnectivityManager.NetworkCallback {
        PeerHandle mConnectionHandle;

        public WifiAwareNetworkCallback(PeerHandle connectionHandle) {
            this.mConnectionHandle = connectionHandle;
        }

        @Override
        public void onAvailable(@NonNull Network network) {
            synchronized (Server.this) {
                Connection conn = mConnectionsMap.get(mConnectionHandle);
                if (conn == null) {
                    return; // Connection not found
                }
                Log.d("Network:", String.valueOf(mConManager.getNetworkCapabilities(network)));
                conn.net = network;
            }
        }

        @Override
        public void onLost(@NonNull Network network) {
            mConManager.unregisterNetworkCallback(this);
            removeConnection(mConnectionHandle);
        }
    }
}
