package pt.iris.network.lib;

import android.net.wifi.aware.PeerHandle;
import android.util.Log;

import java.net.InetAddress;
import java.net.UnknownHostException;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.CopyOnWriteArraySet;

/**
 * Routing table implementation for Wi-Fi Aware mesh network
 * Maps IPv6 addresses to peer handles to avoid broadcasting
 */
public class RoutingTable {
    private static final String TAG = "RoutingTable";
    
    // Map: IPv6 address -> PeerHandle
    private final ConcurrentMap<String, PeerHandle> ipToPeerMap = new ConcurrentHashMap<>();
    
    // Map: PeerHandle -> IPv6 address (reverse lookup)
    private final ConcurrentMap<PeerHandle, String> peerToIpMap = new ConcurrentHashMap<>();
    
    // Map: PeerHandle -> Set of IPv6 addresses (for peers with multiple IPs)
    private final ConcurrentMap<PeerHandle, Set<String>> peerToIpsMap = new ConcurrentHashMap<>();
    
    // Entry timeout in milliseconds (5 minutes)
    private static final long ENTRY_TIMEOUT_MS = 5 * 60 * 1000;
    
    // Map: IPv6 address -> timestamp of last update
    private final ConcurrentMap<String, Long> entryTimestamps = new ConcurrentHashMap<>();
    
    // ULA prefix for validation
    private static final String ULA_PREFIX = "fd00:1234:";

    /**
     * Adds or updates a routing entry
     * @param ipAddress IPv6 address (TUN IP)
     * @param peerHandle PeerHandle associated with this IP
     */
    public synchronized void addRoute(String ipAddress, PeerHandle peerHandle) {
        if (ipAddress == null || peerHandle == null) {
            Log.w(TAG, "Cannot add route with null IP or peer");
            return;
        }
        
        // Normalize IPv6 address
        String normalizedIp = normalizeIpv6Address(ipAddress);
        
        // Validate it's in our ULA subnet
        if (!normalizedIp.startsWith(ULA_PREFIX)) {
            Log.w(TAG, "IP address " + normalizedIp + " is not in ULA subnet, ignoring");
            return;
        }
        
        // Remove old mapping if IP was associated with different peer
        PeerHandle oldPeer = ipToPeerMap.get(normalizedIp);
        if (oldPeer != null && !oldPeer.equals(peerHandle)) {
            removeIpFromPeer(normalizedIp, oldPeer);
        }
        
        // Add new mapping
        ipToPeerMap.put(normalizedIp, peerHandle);
        peerToIpMap.put(peerHandle, normalizedIp);
        
        // Add to peer's IP set
        peerToIpsMap.computeIfAbsent(peerHandle, k -> new CopyOnWriteArraySet<>()).add(normalizedIp);
        
        // Update timestamp
        entryTimestamps.put(normalizedIp, System.currentTimeMillis());
        
        Log.d(TAG, "Added route: " + normalizedIp + " -> " + peerHandle);
    }
    
    /**
     * Gets the peer handle for a given IPv6 address
     * @param ipAddress IPv6 address to lookup
     * @return PeerHandle if found, null otherwise
     */
    public PeerHandle getPeerForIp(String ipAddress) {
        if (ipAddress == null) return null;
        
        String normalizedIp = normalizeIpv6Address(ipAddress);
        PeerHandle peer = ipToPeerMap.get(normalizedIp);
        
        if (peer != null) {
            // Check if entry is expired
            Long timestamp = entryTimestamps.get(normalizedIp);
            if (timestamp != null && System.currentTimeMillis() - timestamp > ENTRY_TIMEOUT_MS) {
                Log.d(TAG, "Route expired for " + normalizedIp + ", removing");
                removeRoute(normalizedIp);
                return null;
            }
        }
        
        return peer;
    }
    
    /**
     * Gets the primary IPv6 address for a given peer
     * @param peerHandle PeerHandle to lookup
     * @return IPv6 address if found, null otherwise
     */
    public String getIpForPeer(PeerHandle peerHandle) {
        return peerToIpMap.get(peerHandle);
    }
    
    /**
     * Gets all IPv6 addresses for a given peer
     * @param peerHandle PeerHandle to lookup
     * @return Set of IPv6 addresses, empty set if none found
     */
    public Set<String> getAllIpsForPeer(PeerHandle peerHandle) {
        Set<String> ips = peerToIpsMap.get(peerHandle);
        return ips != null ? new CopyOnWriteArraySet<>(ips) : new CopyOnWriteArraySet<>();
    }
    
    /**
     * Removes a specific route
     * @param ipAddress IPv6 address to remove
     */
    public synchronized void removeRoute(String ipAddress) {
        if (ipAddress == null) return;
        
        String normalizedIp = normalizeIpv6Address(ipAddress);
        PeerHandle peer = ipToPeerMap.remove(normalizedIp);
        
        if (peer != null) {
            removeIpFromPeer(normalizedIp, peer);
            entryTimestamps.remove(normalizedIp);
            Log.d(TAG, "Removed route: " + normalizedIp + " -> " + peer);
        }
    }
    
    /**
     * Removes all routes for a specific peer
     * @param peerHandle PeerHandle to remove
     */
    public synchronized void removePeer(PeerHandle peerHandle) {
        Set<String> ips = peerToIpsMap.remove(peerHandle);
        if (ips != null) {
            for (String ip : ips) {
                ipToPeerMap.remove(ip);
                entryTimestamps.remove(ip);
            }
            Log.d(TAG, "Removed all routes for peer: " + peerHandle + " (" + ips.size() + " IPs)");
        }
        peerToIpMap.remove(peerHandle);
    }
    
    /**
     * Updates the timestamp for an existing route (for keepalive)
     * @param ipAddress IPv6 address to refresh
     */
    public void refreshRoute(String ipAddress) {
        if (ipAddress == null) return;
        
        String normalizedIp = normalizeIpv6Address(ipAddress);
        if (ipToPeerMap.containsKey(normalizedIp)) {
            entryTimestamps.put(normalizedIp, System.currentTimeMillis());
            Log.v(TAG, "Refreshed route timestamp for: " + normalizedIp);
        }
    }
    
    /**
     * Cleans up expired entries
     */
    public synchronized void cleanupExpiredEntries() {
        long currentTime = System.currentTimeMillis();
        int removedCount = 0;
        
        for (Map.Entry<String, Long> entry : entryTimestamps.entrySet()) {
            if (currentTime - entry.getValue() > ENTRY_TIMEOUT_MS) {
                removeRoute(entry.getKey());
                removedCount++;
            }
        }
        
        if (removedCount > 0) {
            Log.d(TAG, "Cleaned up " + removedCount + " expired routing entries");
        }
    }
    
    /**
     * Gets all current routes for debugging
     * @return Map of IP addresses to PeerHandles
     */
    public Map<String, PeerHandle> getAllRoutes() {
        return new ConcurrentHashMap<>(ipToPeerMap);
    }
    
    /**
     * Gets the total number of routes
     * @return Number of routes
     */
    public int getRouteCount() {
        return ipToPeerMap.size();
    }
    
    /**
     * Checks if the routing table is empty
     * @return true if empty, false otherwise
     */
    public boolean isEmpty() {
        return ipToPeerMap.isEmpty();
    }
    
    /**
     * Clears all routes
     */
    public synchronized void clear() {
        ipToPeerMap.clear();
        peerToIpMap.clear();
        peerToIpsMap.clear();
        entryTimestamps.clear();
        Log.d(TAG, "Routing table cleared");
    }
    
    /**
     * Normalizes IPv6 address to standard format
     * @param ipAddress IPv6 address string
     * @return Normalized IPv6 address
     */
    private String normalizeIpv6Address(String ipAddress) {
        try {
            InetAddress addr = InetAddress.getByName(ipAddress);
            return addr.getHostAddress().toLowerCase();
        } catch (UnknownHostException e) {
            Log.w(TAG, "Failed to normalize IPv6 address: " + ipAddress, e);
            return ipAddress.toLowerCase();
        }
    }
    
    /**
     * Helper method to remove IP from peer's IP set
     */
    private void removeIpFromPeer(String ip, PeerHandle peer) {
        Set<String> ips = peerToIpsMap.get(peer);
        if (ips != null) {
            ips.remove(ip);
            if (ips.isEmpty()) {
                peerToIpsMap.remove(peer);
                peerToIpMap.remove(peer);
            } else if (ip.equals(peerToIpMap.get(peer))) {
                // If we removed the primary IP, set a new one
                String newPrimaryIp = ips.iterator().next();
                peerToIpMap.put(peer, newPrimaryIp);
            }
        }
    }
    
    @Override
    public String toString() {
        return "RoutingTable{" +
                "routes=" + ipToPeerMap.size() +
                ", peers=" + peerToIpMap.size() +
                '}';
    }
}
