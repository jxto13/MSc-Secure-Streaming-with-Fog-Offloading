package com.example.final_test_wifiaware.network.utils;

import java.nio.ByteBuffer;
import java.util.UUID;

public class Utils {
	public static byte[] UUIDtoByteArray(UUID input){
		ByteBuffer uuidByteBuffer = ByteBuffer.wrap(new byte[16]);
		uuidByteBuffer.putLong(input.getMostSignificantBits());
		uuidByteBuffer.putLong(input.getLeastSignificantBits());
		return uuidByteBuffer.array();
	}

	public static UUID ByteArraytoUUID(byte[] input){
		ByteBuffer wrapped = ByteBuffer.wrap(input);
		return new UUID(wrapped.getLong(), wrapped.getLong());
	}

	public static byte[] StringToBytes(String input){
		return input.getBytes();
	}
	public static UUID StringToUUID(String input){
		return UUID.fromString(input);
	}


}
