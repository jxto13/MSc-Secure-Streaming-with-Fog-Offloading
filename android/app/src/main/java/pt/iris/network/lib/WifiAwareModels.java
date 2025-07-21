
package pt.iris.network.lib;

import android.net.Network;
import android.net.wifi.aware.PeerHandle;

import java.net.InetAddress;
import java.nio.ByteBuffer;
import java.util.UUID;

/**
 * Data models for Wi-Fi Aware network management
 */
public class WifiAwareModels {

    /**
     * Data path information holder
     */
    public static class DataPathInfo {
        public final PeerHandle peerHandle;
        public final Network network;
        public final String ipAddress;
        public final long createdTime;
        public boolean isActive;
        public boolean isConnected;
        public UUID peerUuid; // Add peer UUID for display

        public DataPathInfo(PeerHandle peerHandle, Network network, String ipAddress) {
            this.peerHandle = peerHandle;
            this.network = network;
            this.ipAddress = ipAddress;
            this.createdTime = System.currentTimeMillis();
            this.isActive = true;
            this.isConnected = true;
            this.peerUuid = null;

        }

        // Constructor for discovered but not connected peers
        public DataPathInfo(PeerHandle peerHandle) {
            this.peerHandle = peerHandle;
            this.network = null;
            this.ipAddress = "Not Connected";
            this.createdTime = System.currentTimeMillis();
            this.isActive = false;
            this.isConnected = false;
            this.peerUuid = null;
        }
        
        public String getDisplayAddress() {
            if (!isConnected) {
                return "Not Connected";
            }
            return ipAddress;
        }

        public String getInterfaceString() {

            String[] parts = ipAddress.split("%");
            if (parts.length == 2) {
                return parts[1];
            } else {
                return "aware_data0";
            }
        }

        public String getConnectionStatus() {
            if (!isConnected) {
                return "Discovered";
            }
            return isActive ? "Connected" : "Disconnected";
        }
        
        public boolean isConnected() {
            return isConnected;
        }
        
        public long getDurationMs() {
            return System.currentTimeMillis() - createdTime;
        }
        
        public String getDurationString() {
            long durationMs = getDurationMs();
            long seconds = durationMs / 1000;
            long minutes = seconds / 60;
            long remainingSeconds = seconds % 60;
            
            if (minutes > 0) {
                return String.format("%02d:%02d", minutes, remainingSeconds);
            } else {
                return String.format("00:%02d", remainingSeconds);
            }
        }
    }

    /**
     * Device capabilities for role negotiation
     */
    public static class DeviceCapabilities {
        public final int availableDataPaths;
        public UUID deviceUUID;
        public final long timestamp;

        public DeviceCapabilities(int availableDataPaths, UUID deviceUUID) {
            this.availableDataPaths = availableDataPaths;
            this.deviceUUID = deviceUUID;
            this.timestamp = System.currentTimeMillis();
        }

        public boolean canBeMaster() {
            return availableDataPaths > 0;
        }

        public boolean canCreateConnection() {
            return availableDataPaths > 0;
        }

        // Serialize capabilities to byte array for transmission
        public byte[] toBytes() {
            ByteBuffer buffer = ByteBuffer.allocate( 4 + 16 + 8); // int + int + UUID + long
            buffer.putInt(availableDataPaths);
            buffer.putLong(deviceUUID.getMostSignificantBits());
            buffer.putLong(deviceUUID.getLeastSignificantBits());
            buffer.putLong(timestamp);
            return buffer.array();
        }

        // Deserialize capabilities from byte array
        public static DeviceCapabilities fromBytes(byte[] data) {
            if (data.length != 32) return null; // Invalid data

            ByteBuffer buffer = ByteBuffer.wrap(data);
            int interfaces = buffer.getInt();
            long uuidMost = buffer.getLong();
            long uuidLeast = buffer.getLong();
            UUID uuid = new UUID(uuidMost, uuidLeast);

            return new DeviceCapabilities(interfaces, uuid);
        }
    }

    /**
     * Device role enumeration
     */
    public enum DeviceRole {
        UNKNOWN, MASTER, SLAVE
    }

    /**
     * Wi-Fi Aware session state enumeration
     */
    public enum SessionState {
        IDLE,           // Not publishing or subscribing
        DISCOVERING,    // Either publishing or subscribing (but not both)
        ACTIVE          // Both publishing and subscribing
    }

    /**
     * Connection status information for display
     */
    public static class ConnectionInfo {
        public final String deviceId;
        public final String ipAddress;
        public final long createdTime;
        public final boolean isActive;
        public final String myRole;
        public final String peerRole;

        public ConnectionInfo(String deviceId, String ipAddress, long createdTime,
                            boolean isActive, String myRole, String peerRole) {
            this.deviceId = deviceId;
            this.ipAddress = ipAddress;
            this.createdTime = createdTime;
            this.isActive = isActive;
            this.myRole = myRole;
            this.peerRole = peerRole;
        }
        
        public String getDisplayDeviceId() {
            return deviceId != null ? deviceId.substring(0, Math.min(deviceId.length(), 8)) + "..." : "Unknown";
        }
        
        public String getDisplayIpAddress() {
            if (ipAddress != null && ipAddress.startsWith("fe80::")) {
                return ipAddress.substring(0, Math.min(ipAddress.length(), 15)) + "...";
            }
            return ipAddress;
        }
        
        public String getDurationString() {
            long durationMs = System.currentTimeMillis() - createdTime;
            long seconds = durationMs / 1000;
            long minutes = seconds / 60;
            long remainingSeconds = seconds % 60;
            
            if (minutes > 0) {
                return String.format("%02d:%02d", minutes, remainingSeconds);
            } else {
                return String.format("00:%02d", remainingSeconds);
            }
        }
    }

    /**
     * Callback interface for Rust layer integration
     */
    public interface RustServiceCallback {
        void addSocket(String ipAddress);
        void removeSocket(String ipAddress);
    }
    
    /**
     * Wi-Fi Aware session information including role and status
     */
    public static class SessionInfo {
        public final SessionState sessionState;
        public final int discoveredPeers;
        public final int connectedPeers;
        public final long sessionStartTime;

        public SessionInfo(SessionState sessionState,
                          int discoveredPeers, int connectedPeers) {
            this.sessionState = sessionState;
            this.discoveredPeers = discoveredPeers;
            this.connectedPeers = connectedPeers;
            this.sessionStartTime = System.currentTimeMillis();
        }

    }

    /**
     * Message types for Wi-Fi Aware communication
     */
    public static class MessageType {
        public static final String HELLO = "HELLO";
        public static final String CONNECT_REQUEST = "CONNECT_REQUEST";
        public static final String CONNECT_RESPONSE = "CONNECT_RESPONSE";
        public static final String DISCONNECT = "DISCONNECT";
        public static final String CAPABILITY_EXCHANGE = "CAPABILITY_EXCHANGE";
        public static final String HEARTBEAT = "HEARTBEAT";
    }
}