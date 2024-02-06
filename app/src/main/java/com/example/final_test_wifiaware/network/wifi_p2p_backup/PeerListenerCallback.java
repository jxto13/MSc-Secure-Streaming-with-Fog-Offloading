package com.example.final_test_wifiaware.network.wifi_p2p_backup;

import android.net.wifi.p2p.WifiP2pDevice;
import android.net.wifi.p2p.WifiP2pDeviceList;
import android.net.wifi.p2p.WifiP2pManager;
import android.util.Log;

public class PeerListenerCallback implements WifiP2pManager.PeerListListener {
	private String TAG = "PeerListenerCallback";
	@Override
	public void onPeersAvailable(WifiP2pDeviceList peers) {
		int num = 0;
		for (WifiP2pDevice device : peers.getDeviceList()) {
			if(num == 0){
				Log.d(TAG, "List of Peers:\nPeer: " + device.deviceName + " (" + device.deviceAddress + ")");
				num++;
			}else{
				Log.d(TAG, "Peer: " + device.deviceName + " (" + device.deviceAddress + ")");
			}
		}
	}
}
