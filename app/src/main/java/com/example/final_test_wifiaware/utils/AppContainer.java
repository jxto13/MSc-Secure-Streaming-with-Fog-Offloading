package com.example.final_test_wifiaware.utils;

import com.example.final_test_wifiaware.network.connections.ConnectionManager;
import com.example.final_test_wifiaware.network.publisher.Publisher;
import com.example.final_test_wifiaware.network.subscriber.Subscriber;
import com.example.final_test_wifiaware.network.utils.Utils;
import com.example.final_test_wifiaware.services.GrpcClient;
import com.example.final_test_wifiaware.ui.home.HomeViewModel;

import java.util.UUID;

public class AppContainer {
	private static AppContainer instance;
	private UUID myUUID;
	private final HomeViewModel homeViewModel;
	private Subscriber subscriber;
	private Publisher publisher;
	private GrpcClient grpcClient;
	private ConnectionManager connectionManager;
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

	public Subscriber getSubscriber() {
		return subscriber;
	}

	public void setSubscriber(Subscriber subscriber) {
		this.subscriber = subscriber;
	}

	public GrpcClient getGrpcClient() {
		return grpcClient;
	}

	public void setGrpcClient(GrpcClient grpcClient) {
		this.grpcClient = grpcClient;
	}

	public ConnectionManager getConnectionManager() {
		return connectionManager;
	}

	public void setConnectionManager(ConnectionManager connectionManager) {
		this.connectionManager = connectionManager;
	}

	public UUID getMyUUID() {
		return myUUID;
	}

	public void setMyUUID(UUID myUUID) {
		this.myUUID = myUUID;
	}

	public Publisher getPublisher() {
		return publisher;
	}

	public void setPublisher(Publisher publisher) {
		this.publisher = publisher;
	}
}

