package com.example.final_test_wifiaware.network.utils;

import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.wifi.aware.PeerHandle;

import java.util.UUID;
public class Connection {
	private UUID uuid;
	private PeerHandle target;
	private Boolean connected;
	private Network network;
	private NetworkCapabilities networkCapabilities;

	private ConnectivityManager.NetworkCallback connectivityManagerCallback;

	public Connection(UUID uuid, PeerHandle client) {
		this.uuid = uuid;
		this.target = client;
		this.connected = false;
	}

	@Override
	public String toString() {
		return "Connection{" +
				"uuid=" + uuid.toString() +
				"connected=" + connected +
				", connectedClient=" + target.hashCode() +
				", subscriberConnectivityManagerCallback=" +
				'}';
	}

	// Getters and Setters
	public UUID getUUID() {
		return uuid;
	}

	public ConnectivityManager.NetworkCallback getConnectivityManagerCallback() {
		return connectivityManagerCallback;
	}

	public void setConnectivityManagerCallback(ConnectivityManager.NetworkCallback connectivityManagerCallback) {
		this.connectivityManagerCallback = connectivityManagerCallback;
	}

	public PeerHandle getTarget() {
		return target;
	}

	public void setNetwork(Network network) {
		this.network = network;
	}

	public void setNetworkCapabilities(NetworkCapabilities networkCapabilities) {
		this.networkCapabilities = networkCapabilities;
	}

	public Boolean getConnectedStatus() {
		return connected;
	}
}