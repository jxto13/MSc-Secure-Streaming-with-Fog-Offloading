package pt.iris.wifiaware;

import android.net.wifi.aware.PeerHandle;

import java.net.Network;
import java.util.UUID;

/**
 * Data models for WiFi Aware network management
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

        public DataPathInfo(PeerHandle peerHandle, Network network, String ipAddress) {
            this.peerHandle = peerHandle;
            this.network = network;
            this.ipAddress = ipAddress;
            this.createdTime = System.currentTimeMillis();
            this.isActive = true;
        }
    }

    /**
     * Device capabilities for role negotiation
     */
    public static class DeviceCapabilities {
        public final int availableInterfaces;
        public final int availableDataPaths;
        public final UUID deviceUUID;
        public final long timestamp;

        public DeviceCapabilities(int availableInterfaces, int availableDataPaths, UUID deviceUUID) {
            this.availableInterfaces = availableInterfaces;
            this.availableDataPaths = availableDataPaths;
            this.deviceUUID = deviceUUID;
            this.timestamp = System.currentTimeMillis();
        }

        public boolean canBeMaster() {
            return availableInterfaces > 0 && availableDataPaths > 0;
        }

        public boolean canCreateConnection() {
            return availableDataPaths > 0;
        }

        // Serialize capabilities to byte array for transmission
        public byte[] toBytes() {
            java.nio.ByteBuffer buffer = java.nio.ByteBuffer.allocate(4 + 4 + 16 + 8); // int + int + UUID + long
            buffer.putInt(availableInterfaces);
            buffer.putInt(availableDataPaths);
            buffer.putLong(deviceUUID.getMostSignificantBits());
            buffer.putLong(deviceUUID.getLeastSignificantBits());
            buffer.putLong(timestamp);
            return buffer.array();
        }

        // Deserialize capabilities from byte array
        public static DeviceCapabilities fromBytes(byte[] data) {
            if (data.length != 32) return null; // Invalid data

            java.nio.ByteBuffer buffer = java.nio.ByteBuffer.wrap(data);
            int interfaces = buffer.getInt();
            int dataPaths = buffer.getInt();
            long uuidMost = buffer.getLong();
            long uuidLeast = buffer.getLong();
            UUID uuid = new UUID(uuidMost, uuidLeast);

            return new DeviceCapabilities(interfaces, dataPaths, uuid);
        }
    }

    /**
     * Device role enumeration
     */
    public enum DeviceRole {
        UNKNOWN, MASTER, SLAVE
    }

    /**
     * Connection status information
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
    }

    /**
     * Callback interface for Rust layer integration
     */
    public interface RustServiceCallback {
        void addSocket(String ipAddress);
        void removeSocket(String ipAddress);
    }
}
