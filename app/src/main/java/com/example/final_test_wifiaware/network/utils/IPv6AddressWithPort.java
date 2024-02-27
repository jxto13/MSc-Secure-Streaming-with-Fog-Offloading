package com.example.final_test_wifiaware.network.utils;

import java.net.Inet6Address;
import java.util.Objects;

public class IPv6AddressWithPort {
	private Inet6Address address;
	private int port;

	public IPv6AddressWithPort(Inet6Address address, int port) {
		this.address = address;
		this.port = port;
	}

	public Inet6Address getAddress() {
		return address;
	}

	public int getPort() {
		return port;
	}

	@Override
	public boolean equals(Object o) {
		if (this == o) return true;
		if (o == null || getClass() != o.getClass()) return false;
		IPv6AddressWithPort that = (IPv6AddressWithPort) o;
		return port == that.port && address.equals(that.address);
	}

	@Override
	public int hashCode() {
		return Objects.hash(address, port);
	}
}
