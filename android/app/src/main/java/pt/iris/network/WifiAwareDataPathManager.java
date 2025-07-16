package pt.iris.network;

import android.net.wifi.aware.PeerHandle;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * Helper class to bridge NetworkInitializer data with UI components
 */
public class WifiAwareDataPathManager {
    
    private final NetworkInitializer networkInitializer;
    
    public WifiAwareDataPathManager(NetworkInitializer networkInitializer) {
        this.networkInitializer = networkInitializer;
    }
    
    /**
     * Get a list of active WiFi Aware connections for UI display
     */
    public List<WifiAwareConnectionInfo> getActiveConnections() {
        List<WifiAwareConnectionInfo> connections = new ArrayList<>();
        
        if (networkInitializer != null) {
            Map<PeerHandle, NetworkInitializer.DataPathInfo> dataPathMap = 
                networkInitializer.getActiveDataPaths();
            
            for (Map.Entry<PeerHandle, NetworkInitializer.DataPathInfo> entry : dataPathMap.entrySet()) {
                NetworkInitializer.DataPathInfo dataPath = entry.getValue();
                if (dataPath.isActive) {
                    WifiAwareConnectionInfo connectionInfo = new WifiAwareConnectionInfo(
                        entry.getKey().toString(),
                        dataPath.ipAddress,
                        dataPath.createdTime,
                        dataPath.isActive
                    );
                    connections.add(connectionInfo);
                }
            }
        }
        
        return connections;
    }
    
    /**
     * Get data path statistics as a formatted string
     */
    public String getDataPathStatistics() {
        if (networkInitializer != null) {
            return networkInitializer.getDataPathStatistics();
        }
        return "N/A";
    }
    
    /**
     * Information about a WiFi Aware connection for UI display
     */
    public static class WifiAwareConnectionInfo {
        public final String peerHandle;
        public final String ipAddress;
        public final long connectionTime;
        public final boolean isActive;
        public final long duration;
        
        public WifiAwareConnectionInfo(String peerHandle, String ipAddress, 
                                     long connectionTime, boolean isActive) {
            this.peerHandle = peerHandle;
            this.ipAddress = ipAddress;
            this.connectionTime = connectionTime;
            this.isActive = isActive;
            this.duration = System.currentTimeMillis() - connectionTime;
        }
        
        public String getFormattedDuration() {
            long seconds = duration / 1000;
            long minutes = seconds / 60;
            long hours = minutes / 60;
            
            if (hours > 0) {
                return String.format("%02d:%02d:%02d", hours, minutes % 60, seconds % 60);
            } else {
                return String.format("%02d:%02d", minutes, seconds % 60);
            }
        }
        
        public String getShortPeerHandle() {
            if (peerHandle.length() > 8) {
                return peerHandle.substring(0, 8) + "...";
            }
            return peerHandle;
        }
    }
}
