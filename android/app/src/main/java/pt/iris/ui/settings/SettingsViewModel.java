package pt.iris.ui.settings;

import android.content.Context;
import android.content.SharedPreferences;
import androidx.lifecycle.LiveData;
import androidx.lifecycle.MutableLiveData;
import androidx.lifecycle.ViewModel;

import pt.iris.utils.AppContainer;

public class SettingsViewModel extends ViewModel {
    
    private static final String PREFS_NAME = "wifi_aware_settings";
    private static final String KEY_WIFI_AWARE_ENABLED = "wifi_aware_enabled";
    
    private final MutableLiveData<Boolean> wifiAwareEnabled = new MutableLiveData<>();
    private final MutableLiveData<String> nanStatus = new MutableLiveData<>();
    private final MutableLiveData<String> nanActivePaths = new MutableLiveData<>();
    
    private SharedPreferences prefs;
    private Context context;
    
    public SettingsViewModel() {
        // Initialize with default values
        wifiAwareEnabled.setValue(true);
        nanStatus.setValue("NAN Hardware: Checking...");
        nanActivePaths.setValue("Active NAN Paths: 0");
    }
    
    public void initializeWithContext(Context context) {
        this.context = context;
        this.prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
        loadSettings();
        updateNanStatus();
    }
    
    private void loadSettings() {
        if (prefs != null) {
            wifiAwareEnabled.setValue(prefs.getBoolean(KEY_WIFI_AWARE_ENABLED, true));
        }
    }
    
    public void saveSettings() {
        if (prefs != null) {
            SharedPreferences.Editor editor = prefs.edit();
            editor.putBoolean(KEY_WIFI_AWARE_ENABLED, wifiAwareEnabled.getValue());
            editor.apply();
        }
    }
    
    public void updateNanStatus() {
        try {
            AppContainer container = AppContainer.getInstance();
            if (container.getNetworkInitializer() != null) {
                nanStatus.setValue("NAN Hardware: Active");
                // Get active data paths count
                int activePathsCount = container.getNetworkInitializer().getActiveDataPathCount();
                nanActivePaths.setValue("Active NAN Paths: " + activePathsCount);
            } else {
                nanStatus.setValue("NAN Hardware: " + (wifiAwareEnabled.getValue() ? "Available" : "Disabled"));
                nanActivePaths.setValue("Active NAN Paths: 0");
            }
        } catch (Exception e) {
            nanStatus.setValue("NAN Hardware: Error");
            nanActivePaths.setValue("Active NAN Paths: Unknown");
        }
    }
    
    // Getters
    public LiveData<Boolean> getWifiAwareEnabled() { return wifiAwareEnabled; }
    public LiveData<String> getNanStatus() { return nanStatus; }
    public LiveData<String> getNanActivePaths() { return nanActivePaths; }
    
    // Setters
    public void setWifiAwareEnabled(boolean enabled) {
        wifiAwareEnabled.setValue(enabled);
        saveSettings();
        updateNanStatus();
    }
    
    public boolean isWifiAwareEnabled() {
        return wifiAwareEnabled.getValue() != null ? wifiAwareEnabled.getValue() : true;
    }
}
