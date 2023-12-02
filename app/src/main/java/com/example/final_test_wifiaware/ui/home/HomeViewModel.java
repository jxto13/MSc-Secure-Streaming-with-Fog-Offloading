package com.example.final_test_wifiaware.ui.home;

import androidx.lifecycle.LiveData;
import androidx.lifecycle.MutableLiveData;
import androidx.lifecycle.ViewModel;

public class HomeViewModel extends ViewModel {
	private final MutableLiveData<String> myUUID;

	public HomeViewModel() {
		myUUID = new MutableLiveData<>();
	}

	public LiveData<String> getMyUUID() {
		return myUUID;
	}

	public void updateMyUUID(String input) {
		myUUID.setValue(input);
	}
}
