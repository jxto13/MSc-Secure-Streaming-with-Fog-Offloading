package com.example.final_test_wifiaware.utils;

import android.widget.Switch;

import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.services.GrpcClient;
import com.example.final_test_wifiaware.ui.home.HomeViewModel;

import java.util.UUID;

public class AppContainer {
	private static AppContainer instance;
	private UUID myUUID;
	private final HomeViewModel homeViewModel;
	private GrpcClient grpcClient;

	private Switch switchPublisher;
	private Switch switchSubscribe;
	private Switch switchPingAll;

	private AppContainer() {
		homeViewModel = new HomeViewModel();
	}

	public static AppContainer getInstance() {
		if (instance == null) {
			instance = new AppContainer();
		}
		return instance;
	}
	public HomeViewModel getHomeViewModel() {
		return homeViewModel;
	}
	public GrpcClient getGrpcClient() {
		return grpcClient;
	}
	public void setGrpcClient(GrpcClient grpcClient) {
		this.grpcClient = grpcClient;
	}
	public UUID getMyUUID() {
		return myUUID;
	}
	public void setMyUUID(UUID myUUID) {
		this.myUUID = myUUID;
	}

	public void setSwitchPublisher(Switch input){
		switchPublisher = input;
	}
	public void setPingSwitch(Switch pingAll) {
		this.switchPingAll = pingAll;
	}
	public void setSwitchSubscribe(Switch input){
		switchSubscribe = input;
	}

	public Switch getSwitchPublisher() {
		return switchPublisher;
	}

	public Switch getSwitchSubscribe() {
		return switchSubscribe;
	}

	public Switch getSwitchPingAll() {
		return switchPingAll;
	}
}

