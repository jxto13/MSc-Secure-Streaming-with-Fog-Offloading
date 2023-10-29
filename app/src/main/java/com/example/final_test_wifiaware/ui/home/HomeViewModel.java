package com.example.final_test_wifiaware.ui.home;

import android.content.Context;
import android.net.wifi.aware.DiscoverySession;
import android.net.wifi.aware.PeerHandle;
import androidx.lifecycle.LiveData;
import androidx.lifecycle.MutableLiveData;
import androidx.lifecycle.ViewModel;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

public class HomeViewModel extends ViewModel {

	private final MutableLiveData<String> mText;
	private final MutableLiveData<List<String>> discoveredDevices = new MutableLiveData<>();
	private DiscoverySession discoverySession; // Used to manage the Wi-Fi Aware session

	public HomeViewModel() {
		mText = new MutableLiveData<>();
		mText.setValue("This is home fragment");
	}

	public LiveData<String> getText() {
		return mText;
	}

}
