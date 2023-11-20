package com.example.final_test_wifiaware.ui.home;

import androidx.lifecycle.LiveData;
import androidx.lifecycle.MutableLiveData;
import androidx.lifecycle.ViewModel;
import com.example.final_test_wifiaware.network.connections.Connection;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.ArrayList;
import java.util.List;

public class HomeViewModel extends ViewModel {
	private final MutableLiveData<List<Connection>> connectionsLiveData;
	private final MutableLiveData<String> myUUID;

	public HomeViewModel() {
		connectionsLiveData = new MutableLiveData<>();
		connectionsLiveData.setValue(new ArrayList<>());
		myUUID = new MutableLiveData<>();
	}

	public LiveData<List<Connection>> getConnectionsLiveData() {
		return connectionsLiveData;
	}

	// Add a method to update the connections list in the ViewModel
	public void updateConnections(List<Connection> newConnections) {
		connectionsLiveData.setValue(newConnections);
	}

	public List<Connection> getConnectionsList() {
		return connectionsLiveData.getValue();
	}

	public LiveData<String> getMyUUID() {
		return myUUID;
	}

	public void updateMyUUID(String input) {
		myUUID.setValue(input);
	}
}
