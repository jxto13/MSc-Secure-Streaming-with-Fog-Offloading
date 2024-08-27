package pt.iris.utils;

//import pt.iris.services.GrpcClient;
import pt.iris.services.RustService;
import pt.iris.ui.home.HomeViewModel;

import java.util.UUID;

public class AppContainer {
	private static AppContainer instance;
	private UUID myUUID;
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

	public UUID getMyUUID() {
		return myUUID;
	}
	public void setMyUUID(UUID myUUID) {
		this.myUUID = myUUID;
	}
}

