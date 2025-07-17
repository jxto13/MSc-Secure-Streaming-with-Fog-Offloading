package pt.iris.utils;

import androidx.lifecycle.ViewModelProvider;

//import pt.iris.services.GrpcClient;
import pt.iris.services.RustService;
import pt.iris.ui.settings.SettingsViewModel;
import pt.iris.network.NetworkInitializer;

import java.util.UUID;

public class AppContainer {
	private static AppContainer instance;
	private UUID myUUID;
	private NetworkInitializer networkInitializer;
	private Runnable networkInitializationCallback;

	private AppContainer() {
		// Initialize components
	}

	public static AppContainer getInstance() {
		if (instance == null) {
			instance = new AppContainer();
		}
		return instance;
	}

	public ViewModelProvider.Factory getSettingsViewModelFactory() {
		return new ViewModelProvider.Factory() {
			@Override
			public <T extends androidx.lifecycle.ViewModel> T create(Class<T> modelClass) {
				if (modelClass.isAssignableFrom(SettingsViewModel.class)) {
					return (T) new SettingsViewModel();
				}
				throw new IllegalArgumentException("Unknown ViewModel class");
			}
		};
	}

	public UUID getMyUUID() {
		return myUUID;
	}
	
	public void setMyUUID(UUID myUUID) {
		this.myUUID = myUUID;
	}
	
	public NetworkInitializer getNetworkInitializer() {
		return networkInitializer;
	}
	
	public void setNetworkInitializer(NetworkInitializer networkInitializer) {
		this.networkInitializer = networkInitializer;
	}
	
	public void setNetworkInitializationCallback(Runnable callback) {
		this.networkInitializationCallback = callback;
	}
	
	public void triggerNetworkInitialization() {
		if (networkInitializationCallback != null) {
			networkInitializationCallback.run();
		}
	}
}

