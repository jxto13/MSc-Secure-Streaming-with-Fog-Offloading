package pt.iris;


import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.view.Menu;

import pt.iris.R;

import pt.iris.network.NetworkInitializer;
//import pt.iris.services.GrpcClient;
import pt.iris.services.RustService;
import pt.iris.utils.AppContainer;

import com.google.android.material.navigation.NavigationView;

import androidx.navigation.NavController;
import androidx.navigation.Navigation;
import androidx.navigation.ui.AppBarConfiguration;
import androidx.navigation.ui.NavigationUI;
import androidx.drawerlayout.widget.DrawerLayout;
import androidx.appcompat.app.AppCompatActivity;

import pt.iris.databinding.ActivityMainBinding;
import pt.iris.utils.PermissionHandler;

import java.util.UUID;


public class MainActivity extends AppCompatActivity {

	public NetworkInitializer wifiAwareNetwork;
	private AppBarConfiguration mAppBarConfiguration;
	private AppContainer appContainer;
	private Intent serviceIntent;

	private static final String PREFS_NAME = "MyPrefsFile";
	private static final String UUID_KEY = "uuid";

	@Override
	protected void onCreate(Bundle savedInstanceState) {
		super.onCreate(savedInstanceState);

		// Generate or retrieve the UUID
		String uuid = getOrCreateUUID();

		// Set global myUUID in appContainer
		AppContainer.getInstance().setMyUUID(UUID.fromString(uuid));

		// Save the UUID to SharedPreferences
		saveUUIDToSharedPreferences(uuid);

		// Check and request all permissions from the manifest
		PermissionHandler.requestAppPermissions(this);

		pt.iris.databinding.ActivityMainBinding binding = ActivityMainBinding.inflate(getLayoutInflater());
		setContentView(binding.getRoot());

		setSupportActionBar(binding.appBarMain.toolbar);

		DrawerLayout drawer = binding.drawerLayout;
		NavigationView navigationView = binding.navView;

		// Passing each menu ID as a set of Ids because each
		// menu should be considered as top level destinations.
		mAppBarConfiguration = new AppBarConfiguration.Builder(
				R.id.nav_home, R.id.nav_gallery, R.id.nav_slideshow)
				.setOpenableLayout(drawer)
				.build();
		NavController navController = Navigation.findNavController(this, R.id.nav_host_fragment_content_main);
		NavigationUI.setupActionBarWithNavController(this, navController, mAppBarConfiguration);
		NavigationUI.setupWithNavController(navigationView, navController);

		// Start Wifi Aware with NetworkInitializer
		wifiAwareNetwork = new NetworkInitializer(getApplicationContext());
		wifiAwareNetwork.start();

		// Create an instance of the AppContainer
		appContainer = AppContainer.getInstance();

		// Start rustService
		serviceIntent = new Intent(this, RustService.class);
		this.startService(serviceIntent);
	}

	@Override
	public boolean onCreateOptionsMenu(Menu menu) {
		// Inflate the menu; this adds items to the action bar if it is present.
		getMenuInflater().inflate(R.menu.main, menu);
		return true;
	}

	@Override
	public boolean onSupportNavigateUp() {
		NavController navController = Navigation.findNavController(this, R.id.nav_host_fragment_content_main);
		return NavigationUI.navigateUp(navController, mAppBarConfiguration)
				|| super.onSupportNavigateUp();
	}

	private String getOrCreateUUID() {
		SharedPreferences preferences = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
		String uuid = preferences.getString(UUID_KEY, null);

		if (uuid == null) {
			// Generate a new UUID
			uuid = UUID.randomUUID().toString();
		}

		return uuid;
	}

	private void saveUUIDToSharedPreferences(String uuid) {
		SharedPreferences preferences = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
		SharedPreferences.Editor editor = preferences.edit();
		editor.putString(UUID_KEY, uuid);
		editor.apply();
	}

	@Override
	protected void onDestroy() {
		super.onDestroy();
		getDelegate().onDestroy();
		wifiAwareNetwork.closeSessions();
	}

}