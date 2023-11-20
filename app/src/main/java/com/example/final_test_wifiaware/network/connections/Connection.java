package com.example.final_test_wifiaware.network.connections;

import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.wifi.aware.PeerHandle;

import androidx.annotation.NonNull;

import com.example.final_test_wifiaware.network.callbacks.publisher.PublisherConnectivityManagerCallback;
import com.example.final_test_wifiaware.network.callbacks.subscriber.SubscriberConnectivityManagerCallback;
import com.google.errorprone.annotations.FormatString;

import java.util.UUID;
public class Connection {
	private final UUID uuid;
	private PeerHandle peerHandle;
	private Boolean connected;
	private Network network;
	private NetworkCapabilities networkCapabilities;

	private PublisherConnectivityManagerCallback publisherConnectivityManagerCallback;
	private SubscriberConnectivityManagerCallback subscriberConnectivityManagerCallback;
	public Connection(UUID uuid, PeerHandle client) {
		this.uuid = uuid;
		this.peerHandle = client;
		this.connected = false;
	}

	@NonNull
	@Override
	public String toString() {
		return String.format("Connection { uuid = %s, connected = %s, connectedClient = %s}" ,
				uuid.toString(), connected, peerHandle.hashCode());
	}

	// Getters and Setters
	public UUID getUUID() {
		return uuid;
	}

	public PublisherConnectivityManagerCallback getPublisherConnectivityManagerCallback() {
		return publisherConnectivityManagerCallback;
	}

	public void setPublisherConnectivityManagerCallback(PublisherConnectivityManagerCallback connectivityManagerCallback) {
		this.publisherConnectivityManagerCallback = connectivityManagerCallback;
	}

	public SubscriberConnectivityManagerCallback getSubscriberConnectivityManagerCallback() {
		return subscriberConnectivityManagerCallback;
	}

	public void setSubscriberConnectivityManagerCallback(SubscriberConnectivityManagerCallback subscriberConnectivityManagerCallback) {
		this.subscriberConnectivityManagerCallback = subscriberConnectivityManagerCallback;
	}

	public PeerHandle getPeerHandle() {
		return peerHandle;
	}

	public void setPeerHandle(PeerHandle peerHandle) {
		this.peerHandle = peerHandle;
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

	public void setConnectedStatus(Boolean status){
		connected = status;
	}


}
