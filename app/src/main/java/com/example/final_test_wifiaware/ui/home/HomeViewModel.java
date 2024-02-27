package com.example.final_test_wifiaware.ui.home;

import androidx.lifecycle.LiveData;
import androidx.lifecycle.MutableLiveData;
import androidx.lifecycle.ViewModel;

import com.example.final_test_wifiaware.network.utils.Connection;

import java.util.ArrayList;
import java.util.List;

public class HomeViewModel extends ViewModel {
	private final MutableLiveData<String> myUUID;
	private final MutableLiveData<String> subscriberSessionID;
	private final MutableLiveData<String> publisherSessionID;
	private final MutableLiveData<String> clusterID;
	private final MutableLiveData<String> dataPaths;
	private final MutableLiveData<String> sessionPub;
	private final MutableLiveData<String> sessionSub;
	private final MutableLiveData<List<Connection>> connectionsLiveData;
	public HomeViewModel() {
		myUUID = new MutableLiveData<>();
		subscriberSessionID = new MutableLiveData<>();
		publisherSessionID = new MutableLiveData<>();
		clusterID = new MutableLiveData<>();
		dataPaths = new MutableLiveData<>();
		sessionPub = new MutableLiveData<>();
		sessionSub = new MutableLiveData<>();
		connectionsLiveData = new MutableLiveData<>();
		connectionsLiveData.setValue(new ArrayList<>());

	}

	public LiveData<String> getMyUUID() {
		return myUUID;
	}
	public MutableLiveData<String> getClusterID() {
		return clusterID;
	}
	public MutableLiveData<String> getDataPaths() {	return dataPaths; }
	public MutableLiveData<String> getSubscriberSessionID() {
		return subscriberSessionID;
	}
	public MutableLiveData<String> getPublisherSessionID() {
		return publisherSessionID;
	}
	public MutableLiveData<String> getSessionPub() { return sessionPub;	}
	public MutableLiveData<String> getSessionSub() { return sessionSub;	}
	public void updateMyUUID(String input) {
		myUUID.setValue(input);
	}
	public void updateSubscriberSessionID(String input) {
		subscriberSessionID.postValue(input);
	}
	public void updatePublisherSessionID(String input) {
		publisherSessionID.postValue(input);
	}
	public void updateClusterID(String input) {
		clusterID.postValue(input);
	}
	public void updateDataPaths(String input) {
		dataPaths.postValue(input);
	}
	public void updateSessionSub(String input) {
		sessionSub.postValue(input);
	}
	public void updateSessionPub(String input) {sessionPub.postValue(input);}
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


}
