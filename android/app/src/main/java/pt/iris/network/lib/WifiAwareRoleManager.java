package pt.iris.network.lib;

import android.net.wifi.aware.PeerHandle;

import java.util.UUID;

/**
 * Role negotiation utility for WiFi Aware connections
 */
public class WifiAwareRoleManager {

    /**
     * Determine if this device should initiate the connection based on capabilities
     */
    public static boolean shouldInitiateConnection(WifiAwareModels.DeviceCapabilities myCapabilities,
                                                  WifiAwareModels.DeviceCapabilities peerCapabilities) {
        // First priority: device with more available data paths initiates
        if (myCapabilities.availableDataPaths != peerCapabilities.availableDataPaths) {
            return myCapabilities.availableDataPaths > peerCapabilities.availableDataPaths;
        }

        // Second priority: device with more available interfaces initiates
        if (myCapabilities.availableInterfaces != peerCapabilities.availableInterfaces) {
            return myCapabilities.availableInterfaces > peerCapabilities.availableInterfaces;
        }

        // Final tie-breaker: UUID comparison (deterministic)
        return myCapabilities.deviceUUID.compareTo(peerCapabilities.deviceUUID) > 0;
    }

    /**
     * Determine device role based on capabilities and tie-breaking rules
     */
    public static WifiAwareModels.DeviceRole determineMyRole(WifiAwareModels.DeviceCapabilities myCapabilities,
                                                           WifiAwareModels.DeviceCapabilities peerCapabilities) {
        // First priority: device that can be master (has interfaces and data paths)
        boolean iCanBeMaster = myCapabilities.canBeMaster();
        boolean peerCanBeMaster = peerCapabilities.canBeMaster();

        if (iCanBeMaster && !peerCanBeMaster) {
            return WifiAwareModels.DeviceRole.MASTER;
        }

        if (!iCanBeMaster && peerCanBeMaster) {
            return WifiAwareModels.DeviceRole.SLAVE;
        }

        if (!iCanBeMaster && !peerCanBeMaster) {
            return WifiAwareModels.DeviceRole.SLAVE; // Default to slave, connection might not work
        }

        // Both can be master - use tie-breaking rules

        // Second priority: more available data paths
        if (myCapabilities.availableDataPaths > peerCapabilities.availableDataPaths) {
            return WifiAwareModels.DeviceRole.MASTER;
        }

        if (myCapabilities.availableDataPaths < peerCapabilities.availableDataPaths) {
            return WifiAwareModels.DeviceRole.SLAVE;
        }

        // Third priority: more available interfaces
        if (myCapabilities.availableInterfaces > peerCapabilities.availableInterfaces) {
            return WifiAwareModels.DeviceRole.MASTER;
        }

        if (myCapabilities.availableInterfaces < peerCapabilities.availableInterfaces) {
            return WifiAwareModels.DeviceRole.SLAVE;
        }

        // Final tie-breaker: UUID comparison (deterministic)
        int uuidComparison = myCapabilities.deviceUUID.compareTo(peerCapabilities.deviceUUID);
        return (uuidComparison > 0) ? WifiAwareModels.DeviceRole.MASTER : WifiAwareModels.DeviceRole.SLAVE;
    }
}
