package pt.iris.network.utils;

import java.nio.ByteBuffer;
import java.util.UUID;

public class Utils {
	public static UUID asUuid(byte[] bytes) {
		ByteBuffer bb = ByteBuffer.wrap(bytes);
		long firstLong = bb.getLong();
		long secondLong = bb.getLong();
		return new UUID(firstLong, secondLong);
	}
	public static byte[] asBytes(UUID uuid) {
		ByteBuffer bb = ByteBuffer.wrap(new byte[16]);
		bb.putLong(uuid.getMostSignificantBits());
		bb.putLong(uuid.getLeastSignificantBits());
		return bb.array();
	}

	public static boolean isMyUuidLower(UUID myUuid, UUID otherUuid) {
		int compareMost = Long.compare(myUuid.getMostSignificantBits(), otherUuid.getMostSignificantBits());
		if (compareMost < 0) {
			return true;
		} else if (compareMost > 0) {
			return false;
		} else {
			// Most significant bits are equal, compare least significant
			return Long.compare(myUuid.getLeastSignificantBits(), otherUuid.getLeastSignificantBits()) < 0;
		}
	}



}
