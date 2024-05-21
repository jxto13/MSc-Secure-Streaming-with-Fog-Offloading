package pt.iris.utils;

//import pt.iris.services.GrpcClient;
import pt.iris.services.RustService;
import pt.iris.ui.home.HomeViewModel;

import java.util.UUID;

public class AppContainer {
	private static AppContainer instance;
	private UUID myUUID;
	private final HomeViewModel homeViewModel;
	//private GrpcClient grpcClient;
	private RustService rustService;

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
	/*public GrpcClient getGrpcClient() {
		return grpcClient;
	}*/

	public RustService getRustService() {
		return rustService;
	}

	public void setRustService(RustService rustService) {
		this.rustService = rustService;
	}

	/*public void setGrpcClient(GrpcClient grpcClient) {
		this.grpcClient = grpcClient;
	}*/
	public UUID getMyUUID() {
		return myUUID;
	}
	public void setMyUUID(UUID myUUID) {
		this.myUUID = myUUID;
	}
}

