package pt.iris.network.lib;

import android.util.Log;

import java.net.InetAddress;
import java.net.UnknownHostException;
import java.nio.ByteBuffer;

/**
 * Utility class for parsing IPv6 packets
 */
public class IPv6PacketParser {
    private static final String TAG = "IPv6PacketParser";
    
    // IPv6 header field offsets
    private static final int VERSION_OFFSET = 0;
    private static final int PAYLOAD_LENGTH_OFFSET = 4;
    private static final int NEXT_HEADER_OFFSET = 6;
    private static final int HOP_LIMIT_OFFSET = 7;
    private static final int SOURCE_ADDRESS_OFFSET = 8;
    private static final int DESTINATION_ADDRESS_OFFSET = 24;
    
    // IPv6 header length
    private static final int IPV6_HEADER_LENGTH = 40;
    
    // Protocol numbers
    public static final int PROTO_ICMPV6 = 58;
    public static final int PROTO_TCP = 6;
    public static final int PROTO_UDP = 17;
    
    /**
     * Represents a parsed IPv6 packet
     */
    public static class IPv6Packet {
        public final String sourceAddress;
        public final String destinationAddress;
        public final int nextHeader;
        public final int payloadLength;
        public final int hopLimit;
        public final byte[] data;
        public final boolean isValid;
        
        public IPv6Packet(String sourceAddress, String destinationAddress, 
                         int nextHeader, int payloadLength, int hopLimit, 
                         byte[] data, boolean isValid) {
            this.sourceAddress = sourceAddress;
            this.destinationAddress = destinationAddress;
            this.nextHeader = nextHeader;
            this.payloadLength = payloadLength;
            this.hopLimit = hopLimit;
            this.data = data;
            this.isValid = isValid;
        }
        
        public String getProtocolName() {
            switch (nextHeader) {
                case PROTO_TCP: return "TCP";
                case PROTO_UDP: return "UDP";
                case PROTO_ICMPV6: return "ICMPv6";
                default: return "Other(" + nextHeader + ")";
            }
        }
        
        public boolean isICMPv6() {
            return nextHeader == PROTO_ICMPV6;
        }
        
        public boolean isTCP() {
            return nextHeader == PROTO_TCP;
        }
        
        public boolean isUDP() {
            return nextHeader == PROTO_UDP;
        }
        
        @Override
        public String toString() {
            return String.format("IPv6[%s -> %s, %s, %d bytes]", 
                sourceAddress, destinationAddress, getProtocolName(), data.length);
        }
    }
    
    /**
     * Parses an IPv6 packet from byte array
     * @param packetData Raw packet data
     * @param length Length of valid data in the array
     * @return Parsed IPv6Packet object
     */
    public static IPv6Packet parsePacket(byte[] packetData, int length) {
        if (packetData == null || length < IPV6_HEADER_LENGTH) {
            Log.w(TAG, "Packet too short for IPv6 header: " + length + " bytes");
            return new IPv6Packet(null, null, 0, 0, 0, packetData, false);
        }
        
        try {
            // Check version (should be 6 for IPv6)
            int version = (packetData[VERSION_OFFSET] & 0xF0) >> 4;
            if (version != 6) {
                Log.w(TAG, "Not an IPv6 packet, version: " + version);
                return new IPv6Packet(null, null, 0, 0, 0, packetData, false);
            }
            
            // Extract header fields
            int payloadLength = ((packetData[PAYLOAD_LENGTH_OFFSET] & 0xFF) << 8) | 
                               (packetData[PAYLOAD_LENGTH_OFFSET + 1] & 0xFF);
            int nextHeader = packetData[NEXT_HEADER_OFFSET] & 0xFF;
            int hopLimit = packetData[HOP_LIMIT_OFFSET] & 0xFF;
            
            // Extract source address
            byte[] sourceBytes = new byte[16];
            System.arraycopy(packetData, SOURCE_ADDRESS_OFFSET, sourceBytes, 0, 16);
            String sourceAddress = InetAddress.getByAddress(sourceBytes).getHostAddress();
            
            // Extract destination address
            byte[] destBytes = new byte[16];
            System.arraycopy(packetData, DESTINATION_ADDRESS_OFFSET, destBytes, 0, 16);
            String destinationAddress = InetAddress.getByAddress(destBytes).getHostAddress();
            
            // Create a copy of the packet data with the correct length
            byte[] data = new byte[length];
            System.arraycopy(packetData, 0, data, 0, length);
            
            return new IPv6Packet(sourceAddress, destinationAddress, nextHeader, 
                                payloadLength, hopLimit, data, true);
                                
        } catch (UnknownHostException e) {
            Log.e(TAG, "Failed to parse IPv6 addresses", e);
            return new IPv6Packet(null, null, 0, 0, 0, packetData, false);
        } catch (Exception e) {
            Log.e(TAG, "Unexpected error parsing IPv6 packet", e);
            return new IPv6Packet(null, null, 0, 0, 0, packetData, false);
        }
    }
    
    /**
     * Extracts the destination IPv6 address from a packet
     * @param packetData Raw packet data
     * @param length Length of valid data
     * @return Destination IPv6 address as string, null if parsing fails
     */
    public static String extractDestinationAddress(byte[] packetData, int length) {
        IPv6Packet packet = parsePacket(packetData, length);
        return packet.isValid ? packet.destinationAddress : null;
    }
    
    /**
     * Extracts the source IPv6 address from a packet
     * @param packetData Raw packet data
     * @param length Length of valid data
     * @return Source IPv6 address as string, null if parsing fails
     */
    public static String extractSourceAddress(byte[] packetData, int length) {
        IPv6Packet packet = parsePacket(packetData, length);
        return packet.isValid ? packet.sourceAddress : null;
    }
    
    /**
     * Checks if a packet is an ICMPv6 packet
     * @param packetData Raw packet data
     * @param length Length of valid data
     * @return true if ICMPv6, false otherwise
     */
    public static boolean isICMPv6Packet(byte[] packetData, int length) {
        if (length < IPV6_HEADER_LENGTH) return false;
        return (packetData[NEXT_HEADER_OFFSET] & 0xFF) == PROTO_ICMPV6;
    }
    
    /**
     * Checks if an address is in the ULA subnet
     * @param ipAddress IPv6 address string
     * @return true if in ULA subnet, false otherwise
     */
    public static boolean isULAAddress(String ipAddress) {
        return ipAddress != null && ipAddress.toLowerCase().startsWith("fd00:1234:");
    }
    
    /**
     * Checks if an address is a multicast address
     * @param ipAddress IPv6 address string
     * @return true if multicast, false otherwise
     */
    public static boolean isMulticastAddress(String ipAddress) {
        return ipAddress != null && ipAddress.toLowerCase().startsWith("ff");
    }
    
    /**
     * Checks if an address is a link-local address
     * @param ipAddress IPv6 address string
     * @return true if link-local, false otherwise
     */
    public static boolean isLinkLocalAddress(String ipAddress) {
        return ipAddress != null && ipAddress.toLowerCase().startsWith("fe80:");
    }
}
