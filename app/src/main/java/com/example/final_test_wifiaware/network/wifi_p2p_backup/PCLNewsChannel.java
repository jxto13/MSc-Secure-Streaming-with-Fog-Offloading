package com.example.final_test_wifiaware.network.wifi_p2p_backup;

import android.net.wifi.p2p.WifiP2pDevice;

import java.beans.PropertyChangeEvent;
import java.beans.PropertyChangeListener;
import java.util.HashSet;
import java.util.Set;

public class PCLNewsChannel implements PropertyChangeListener {

	private Set<WifiP2pDevice> discoveredPeers = new HashSet<>();

	public void propertyChange(PropertyChangeEvent evt) {
		System.out.println("Peer added");
		this.setNews((WifiP2pDevice) evt.getNewValue());
	}

	public Set<WifiP2pDevice> getNews() {
		return discoveredPeers;
	}

	public void setNews(WifiP2pDevice news) {
		discoveredPeers.add(news);
	}
}