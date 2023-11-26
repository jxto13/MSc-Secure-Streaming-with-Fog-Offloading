package com.example.final_test_wifiaware.network.protocol;

import android.net.wifi.aware.PeerHandle;
import android.util.Log;

import com.example.final_test_wifiaware.network.connections.ConnectionManager;
import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.utils.AppContainer;

public class MessageHandler {
	private final ConnectionManager connectionManager = AppContainer.getInstance().getConnectionManager();

	public void parseMessage(PeerHandle peerHandle, byte[] message){
		// message always in format of "protocolID:uuid"
		String messageString = new String(message);
		String[] splitMessage = messageString.split(":");

		Log.d("Message","received from " + splitMessage[1] + " : " + splitMessage[0]);
		switch (splitMessage[0]){
			case "subscriberID": // message from subscriber to publisher to tell he's a subscriber
				connectionManager.checkConnectionExists(Utils.StringToUUID(splitMessage[1]),peerHandle);
				break;
			case "networkCreation":
				connectionManager.createSubscriberNetwork(Utils.StringToUUID(splitMessage[1]),peerHandle);
				break;
		}
	}
}
