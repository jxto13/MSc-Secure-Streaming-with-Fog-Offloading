package com.example.final_test_wifiaware.utils;

import com.example.final_test_wifiaware.ui.home.HomeViewModel;

public class AppContainer {
	private static AppContainer instance;
	private final HomeViewModel homeViewModel;

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
}

