package pt.iris;

import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.view.Menu;
import android.view.MenuItem;
import android.widget.Toast;
import androidx.annotation.NonNull;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.view.GravityCompat;
import androidx.drawerlayout.widget.DrawerLayout;
import androidx.navigation.NavController;
import androidx.navigation.Navigation;
import androidx.navigation.ui.AppBarConfiguration;
import androidx.navigation.ui.NavigationUI;
import com.google.android.material.navigation.NavigationView;

import pt.iris.databinding.ActivityMainBinding;
import pt.iris.network.NetworkInitializer;
import pt.iris.services.RustService;
import pt.iris.utils.AppContainer;
import pt.iris.utils.CertificateUtil;
import pt.iris.utils.PermissionHandler;

import java.io.IOException;
import java.util.List;
import java.util.UUID;

/**
 * Main Activity for WiFi Aware Video Streaming App
 * Uses navigation drawer to switch between streaming and settings
 */
public class MainActivity extends AppCompatActivity {

    public NetworkInitializer wifiAwareNetwork;
    private AppContainer appContainer;
    private Intent serviceIntent;
    private ActivityMainBinding binding;
    private NavController navController;
    private AppBarConfiguration mAppBarConfiguration;

    private static final String PREFS_NAME = "MyPrefsFile";
    private static final String UUID_KEY = "uuid";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        binding = ActivityMainBinding.inflate(getLayoutInflater());
        setContentView(binding.getRoot());

        // Initialize basic components first
        initializeBasicComponents();

        // Request permissions with enhanced UI
        requestPermissionsWithUI();
    }

    private void initializeBasicComponents() {
        // Copy public key certificate to data folder of app
        try {
            CertificateUtil.copyCertificateToDataFolder(this);
        } catch (IOException e) {
            throw new RuntimeException("Failed to copy certificate", e);
        }

        // Generate or retrieve the UUID
        String uuid = getOrCreateUUID();

        // Set global myUUID in appContainer
        AppContainer.getInstance().setMyUUID(UUID.fromString(uuid));

        // Save the UUID to SharedPreferences
        saveUUIDToSharedPreferences(uuid);

        // Create an instance of the AppContainer
        appContainer = AppContainer.getInstance();

        // Setup toolbar
        setSupportActionBar(binding.appBarMain.toolbar);

        DrawerLayout drawer = binding.drawerLayout;
        NavigationView navigationView = binding.navView;

        // Passing each menu ID as a set of Ids because each
        // menu should be considered as top level destinations.
        mAppBarConfiguration = new AppBarConfiguration.Builder(
                R.id.nav_streaming, R.id.nav_settings)
                .setOpenableLayout(drawer)
                .build();
        
        // Delay NavController setup until view hierarchy is ready
        binding.getRoot().post(() -> setupNavigation(navigationView));
    }

    private void setupNavigation(NavigationView navigationView) {
        try {
            // Find NavController and setup navigation
            navController = Navigation.findNavController(this, R.id.nav_host_fragment_content_main);
            NavigationUI.setupActionBarWithNavController(this, navController, mAppBarConfiguration);
            NavigationUI.setupWithNavController(navigationView, navController);
        } catch (Exception e) {
            // If NavController setup fails, log error and continue
            android.util.Log.e("MainActivity", "Failed to setup navigation: " + e.getMessage(), e);
        }
    }

    @Override
    public boolean onCreateOptionsMenu(Menu menu) {
        // Options menu disabled - using navigation drawer instead
        return false;
    }

    @Override
    public boolean onSupportNavigateUp() {
        if (navController != null) {
            return NavigationUI.navigateUp(navController, mAppBarConfiguration)
                    || super.onSupportNavigateUp();
        }
        return super.onSupportNavigateUp();
    }


    private void requestPermissionsWithUI() {
        PermissionHandler.requestAppPermissions(this, new PermissionHandler.PermissionCallback() {
            @Override
            public void onPermissionsGranted() {
                // All permissions granted, proceed with app initialization
                initializeAppWithPermissions();
            }

            @Override
            public void onPermissionsDenied(List<String> deniedPermissions) {
                // Some permissions denied, show warning but continue
                Toast.makeText(MainActivity.this, 
                    "Some features may not work without all permissions", 
                    Toast.LENGTH_LONG).show();
                initializeAppWithPermissions();
            }

            @Override
            public void onPermissionsPermanentlyDenied(List<String> permanentlyDeniedPermissions) {
                // Critical permissions permanently denied
                Toast.makeText(MainActivity.this, 
                    "App cannot function without required permissions. Please enable them in settings.", 
                    Toast.LENGTH_LONG).show();
                
                // Still try to initialize but with limited functionality
                initializeAppWithPermissions();
            }
        });
    }

    private void initializeAppWithPermissions() {
        // Check WiFi Aware setting before starting
        android.content.SharedPreferences prefs = getSharedPreferences("wifi_aware_settings", Context.MODE_PRIVATE);
        boolean wifiAwareEnabled = prefs.getBoolean("wifi_aware_enabled", true);
        
        // Initialize WiFi Aware immediately after permissions are granted
        initializeWifiAwareNetwork(wifiAwareEnabled);
        
        // Set up network initialization callback via AppContainer for server-dependent features
        AppContainer.getInstance().setNetworkInitializationCallback(() -> {
            // This will be called when the Rust client successfully connects to server
            runOnUiThread(() -> {
                Toast.makeText(MainActivity.this, "Server connected! Ready for streaming...", Toast.LENGTH_SHORT).show();
                // Any server-dependent initialization can go here
            });
        });

        // Prepare Rust service (start when needed)
        serviceIntent = new Intent(this, RustService.class);
    }
    
    private void initializeWifiAwareNetwork(boolean wifiAwareEnabled) {
        // Always start WiFi Aware if permissions are granted (ignoring the setting)
        if (PermissionHandler.areAllPermissionsGranted(this)) {
            wifiAwareNetwork = new NetworkInitializer(getApplicationContext());
            
            // Initialize the Wi-Fi Aware functionality
            wifiAwareNetwork.initialize();
            
            // Store in AppContainer for access by other components
            AppContainer.getInstance().setNetworkInitializer(wifiAwareNetwork);

            Toast.makeText(this, "WiFi Aware initialized at startup", Toast.LENGTH_SHORT).show();
        } else {
            Toast.makeText(this, "WiFi Aware requires all permissions to function", Toast.LENGTH_LONG).show();
        }
    }


    @Override
    public void onRequestPermissionsResult(int requestCode, @NonNull String[] permissions, @NonNull int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        PermissionHandler.onRequestPermissionsResult(this, requestCode, permissions, grantResults);
    }

    @Override
    public boolean onOptionsItemSelected(MenuItem item) {
        // Remove settings option since it's now in the navigation drawer
        return super.onOptionsItemSelected(item);
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
        
        android.util.Log.d("MainActivity", "MainActivity onDestroy called");
        
        // Stop Rust client first - most important for thread cleanup
        stopRustClient();
        
        // Clean up WiFi Aware
        if (wifiAwareNetwork != null) {
            wifiAwareNetwork.destroy();
            wifiAwareNetwork = null;
        }
        
        // Stop Rust service
        if (serviceIntent != null) {
            this.stopService(serviceIntent);
            serviceIntent = null;
        }
        
        // Clean up binding
        binding = null;
        
        android.util.Log.d("MainActivity", "MainActivity onDestroy completed");
    }

    @Override
    protected void onPause() {
        super.onPause();
        // Optionally stop client when app goes to background to save resources
        // Uncomment if you want to stop streaming when app is backgrounded
        // stopRustClient();
    }

    @Override
    protected void onStop() {
        super.onStop();
        // Stop client when activity is not visible
        stopRustClient();
    }

    private void stopRustClient() {
        try {
            RustService.stopClient();
            android.util.Log.d("MainActivity", "Rust client stopped");
        } catch (Exception e) {
            android.util.Log.w("MainActivity", "Error stopping Rust client", e);
        }
    }

    /**
     * Get the AppContainer instance for dependency injection
     */
    public AppContainer getAppContainer() {
        return appContainer;
    }

    @Override
    protected void finalize() throws Throwable {
        try {
            // Last resort cleanup in case onDestroy wasn't called
            android.util.Log.d("MainActivity", "MainActivity finalize called - last resort cleanup");
            stopRustClient();
        } finally {
            super.finalize();
        }
    }
}