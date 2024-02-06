package com.example.final_test_wifiaware.network.wifi_p2p_backup;

import android.net.wifi.p2p.WifiP2pDevice;

import java.beans.PropertyChangeListener;
import java.beans.PropertyChangeSupport;
import java.util.HashSet;
import java.util.Set;

public class PCLNewsAgency {
	private WifiP2pDevice discoveredPeers;
	private PropertyChangeSupport support;

	public PCLNewsAgency() {
		support = new PropertyChangeSupport(this);
	}

	public void addPropertyChangeListener(PropertyChangeListener pcl) {
		support.addPropertyChangeListener(pcl);
	}

	public void removePropertyChangeListener(PropertyChangeListener pcl) {
		support.removePropertyChangeListener(pcl);
	}

	public void setNews(WifiP2pDevice value) {
		support.firePropertyChange("listOfPeers", this.discoveredPeers, value);
		this.discoveredPeers = value;

	}
}