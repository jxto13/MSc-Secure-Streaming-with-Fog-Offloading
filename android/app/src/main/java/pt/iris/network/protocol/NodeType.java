package pt.iris.network.protocol;

public enum NodeType {
	CLIENT,
	RELAY;

	public static NodeType fromInterfaceCount(int interfaceCount) {
		if (interfaceCount == 1) {
			return CLIENT;
		} else {
			return RELAY;
		}
	}
}
