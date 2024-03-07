package com.example.final_test_wifiaware.network.protocol;

public class MessageType {
	// Discovery
	public static final byte[] FOUND_PEER = { (byte) 0x01, (byte) 0x01 }; // Client to Server on Discover
	public static final byte[] ACK_PEER = { (byte) 0x01, (byte) 0x02 }; // ACK from Server to Client

	// Prepare Connection
	public static final byte[] REQUEST_INTERFACE = { (byte) 0x01, (byte) 0x03 }; // Client request interface for connection
	public static final byte[] ACK_INTERFACE = { (byte) 0x01, (byte) 0x04 }; // Server sends ACK interface
	public static final byte[] DENY_INTERFACE = { (byte) 0x01, (byte) 0x05 }; // Server sends Deny interface

	// Start Connection
	public static final byte[] REQUEST_CONNECTION = { (byte) 0x01, (byte) 0x06 }; // Client tells to start Connection (but Server needs to start the network first!)
	public static final byte[] START_CONNECTION = { (byte) 0x01, (byte) 0x07 }; // Client tells to start Connection (but Server needs to start the network first!)
	public static final byte[] STOP_CONNECTION = { (byte) 0x01, (byte) 0x08 }; // Client or Server sends when Network is lost

}
