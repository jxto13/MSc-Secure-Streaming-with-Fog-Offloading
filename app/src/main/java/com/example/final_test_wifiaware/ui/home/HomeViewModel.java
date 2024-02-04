package com.example.final_test_wifiaware.ui.home;

import androidx.lifecycle.LiveData;
import androidx.lifecycle.MutableLiveData;
import androidx.lifecycle.ViewModel;

public class HomeViewModel extends ViewModel {
	private final MutableLiveData<String> myUUID;
	private final MutableLiveData<String> subscriberSessionID;
	private final MutableLiveData<String> publisherSessionID;

	public HomeViewModel() {
		myUUID = new MutableLiveData<>();
		subscriberSessionID = new MutableLiveData<>();
		publisherSessionID = new MutableLiveData<>();
	}

	public LiveData<String> getMyUUID() {
		return myUUID;
	}

	public void updateMyUUID(String input) {
		myUUID.setValue(input);
	}

	public MutableLiveData<String> getSubscriberSessionID() {
		return subscriberSessionID;
	}

	public MutableLiveData<String> getPublisherSessionID() {
		return publisherSessionID;
	}

	public void updateSubscriberSessionID(String input) {
		subscriberSessionID.postValue(input);
	}
	public void updatePublisherSessionID(String input) {
		publisherSessionID.postValue(input);
	}


}
