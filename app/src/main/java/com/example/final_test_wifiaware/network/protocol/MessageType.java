package com.example.final_test_wifiaware.network.protocol;

public class MessageType {
	public static final byte[] START_SEQUENCE = { (byte) 0x01, (byte) 0x01 };
	public static final byte[] START_CONNECTION = { (byte) 0x01, (byte) 0x02 };
	public static final byte[] FOUND_PEER = { (byte) 0x01, (byte) 0x03 };

}
