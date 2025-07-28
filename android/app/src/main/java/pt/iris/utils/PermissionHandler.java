package pt.iris.utils;

import android.Manifest;
import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import androidx.annotation.NonNull;
import androidx.appcompat.app.AlertDialog;
import androidx.core.app.ActivityCompat;
import androidx.core.content.ContextCompat;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * PermissionHandler: Manages Android permissions with customizable UI callbacks.
 */
public class PermissionHandler {

    private static final int PERMISSION_REQUEST_CODE = 1001;
    private static final int SETTINGS_REQUEST_CODE = 1002;

    // Required permissions
    private static final String[] REQUIRED_PERMISSIONS = {
            Manifest.permission.ACCESS_WIFI_STATE,
            Manifest.permission.CHANGE_WIFI_STATE,
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_COARSE_LOCATION,
            Manifest.permission.NEARBY_WIFI_DEVICES, // Android 13+
            Manifest.permission.INTERNET,
            Manifest.permission.ACCESS_NETWORK_STATE,
            Manifest.permission.CHANGE_NETWORK_STATE,
            Manifest.permission.WRITE_EXTERNAL_STORAGE, // Android 12 and below
            Manifest.permission.READ_EXTERNAL_STORAGE,  // Android 12 and below
    };

    // Permission explanations
    private static final Map<String, String> PERMISSION_EXPLANATIONS = new HashMap<String, String>() {{
        put(Manifest.permission.ACCESS_FINE_LOCATION, "Required for WiFi Aware device discovery and precise peer location");
        put(Manifest.permission.ACCESS_COARSE_LOCATION, "Required for WiFi Aware network functionality");
        put(Manifest.permission.ACCESS_WIFI_STATE, "Required to access WiFi Aware capabilities and network state");
        put(Manifest.permission.CHANGE_WIFI_STATE, "Required to manage WiFi Aware connections and data paths");
        put(Manifest.permission.NEARBY_WIFI_DEVICES, "Required for WiFi Aware peer discovery (Android 13+)");
        put(Manifest.permission.INTERNET, "Required for video streaming functionality");
        put(Manifest.permission.ACCESS_NETWORK_STATE, "Required to monitor network connection status");
        put(Manifest.permission.CHANGE_NETWORK_STATE, "Required to manage WiFi Aware data path connections");
        put(Manifest.permission.WRITE_EXTERNAL_STORAGE, "Required for app logging and data storage (Android 12 and below)");
        put(Manifest.permission.READ_EXTERNAL_STORAGE, "Required for reading configuration files (Android 12 and below)");
    }};

    public interface PermissionCallback {
        void onPermissionsGranted();
        void onPermissionsDenied(List<String> deniedPermissions);
        void onPermissionsPermanentlyDenied(List<String> permanentlyDeniedPermissions);
    }

    public interface PermissionUIHandler {
        void showPermissionExplanation(Activity activity, List<String> permissions, Runnable onConfirm, Runnable onCancel);
        void showSettingsRedirect(Activity activity, List<String> permanentlyDeniedPermissions, Runnable onSettings, Runnable onCancel);
        void showRetryDialog(Activity activity, List<String> deniedPermissions, Runnable onRetry, Runnable onCancel);
    }

    private static PermissionCallback currentCallback;
    private static PermissionUIHandler uiHandler;

    // Initialize with custom UI handler (optional)
    public static void initialize(PermissionUIHandler handler) {
        uiHandler = handler;
    }

    // Request permissions
    public static void requestAppPermissions(Activity activity, PermissionCallback callback) {
        currentCallback = callback;
        List<String> missingPermissions = getMissingPermissions(activity);

        if (missingPermissions.isEmpty()) {
            if (callback != null) callback.onPermissionsGranted();
        } else {
            ensureUIHandler();
            uiHandler.showPermissionExplanation(
                    activity,
                    missingPermissions,
                    () -> requestPermissions(activity, missingPermissions),
                    () -> { if (callback != null) callback.onPermissionsDenied(missingPermissions); }
            );
        }
    }

    public static void onRequestPermissionsResult(Activity activity, int requestCode,
                                                  @NonNull String[] permissions,
                                                  @NonNull int[] grantResults) {
        if (requestCode != PERMISSION_REQUEST_CODE) return;

        List<String> denied = new ArrayList<>();
        List<String> permanentlyDenied = new ArrayList<>();

        for (int i = 0; i < permissions.length; i++) {
            if (grantResults[i] != PackageManager.PERMISSION_GRANTED) {
                if (ActivityCompat.shouldShowRequestPermissionRationale(activity, permissions[i])) {
                    denied.add(permissions[i]);
                } else {
                    permanentlyDenied.add(permissions[i]);
                }
            }
        }

        ensureUIHandler();
        if (denied.isEmpty() && permanentlyDenied.isEmpty()) {
            if (currentCallback != null) currentCallback.onPermissionsGranted();
        } else if (!permanentlyDenied.isEmpty()) {
            uiHandler.showSettingsRedirect(
                    activity,
                    permanentlyDenied,
                    () -> openSettings(activity),
                    () -> { if (currentCallback != null) currentCallback.onPermissionsPermanentlyDenied(permanentlyDenied); }
            );
        } else {
            uiHandler.showRetryDialog(
                    activity,
                    denied,
                    () -> requestPermissions(activity, denied),
                    () -> { if (currentCallback != null) currentCallback.onPermissionsDenied(denied); }
            );
        }
    }

    // -------- Helper Methods -------- //

    private static void requestPermissions(Activity activity, List<String> permissions) {
        ActivityCompat.requestPermissions(activity, permissions.toArray(new String[0]), PERMISSION_REQUEST_CODE);
    }

    private static void openSettings(Activity activity) {
        Intent intent = new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS);
        Uri uri = Uri.fromParts("package", activity.getPackageName(), null);
        intent.setData(uri);
        activity.startActivityForResult(intent, SETTINGS_REQUEST_CODE);
    }

    private static List<String> getMissingPermissions(Context context) {
        List<String> missingPermissions = new ArrayList<>();
        for (String permission : REQUIRED_PERMISSIONS) {
            if (permission.equals(Manifest.permission.NEARBY_WIFI_DEVICES) && Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) continue;
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                    (permission.equals(Manifest.permission.READ_EXTERNAL_STORAGE) || permission.equals(Manifest.permission.WRITE_EXTERNAL_STORAGE)))
                continue;
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU &&
                    (permission.equals("android.permission.READ_MEDIA_IMAGES") || permission.equals("android.permission.READ_MEDIA_VIDEO") || permission.equals("android.permission.READ_MEDIA_AUDIO")))
                continue;
            if (ContextCompat.checkSelfPermission(context, permission) != PackageManager.PERMISSION_GRANTED) {
                missingPermissions.add(permission);
            }
        }
        return missingPermissions;
    }

    public static boolean areAllPermissionsGranted(Context context) {
        return getMissingPermissions(context).isEmpty();
    }

    public static String getPermissionDisplayName(String permission) {
        switch (permission) {
            case Manifest.permission.ACCESS_FINE_LOCATION: return "Precise Location";
            case Manifest.permission.ACCESS_COARSE_LOCATION: return "Approximate Location";
            case Manifest.permission.ACCESS_WIFI_STATE: return "WiFi State Access";
            case Manifest.permission.CHANGE_WIFI_STATE: return "WiFi State Changes";
            case Manifest.permission.NEARBY_WIFI_DEVICES: return "Nearby WiFi Devices";
            case Manifest.permission.INTERNET: return "Internet Access";
            case Manifest.permission.ACCESS_NETWORK_STATE: return "Network State Access";
            case Manifest.permission.CHANGE_NETWORK_STATE: return "Network State Changes";
            case Manifest.permission.WRITE_EXTERNAL_STORAGE: return "Storage Write Access";
            case Manifest.permission.READ_EXTERNAL_STORAGE: return "Storage Read Access";
            default: return permission.substring(permission.lastIndexOf('.') + 1);
        }
    }

    private static void ensureUIHandler() {
        if (uiHandler == null) uiHandler = getDefaultUIHandler();
    }

    // -------- Default UI Handler (Material Dialogs) -------- //
    private static PermissionUIHandler getDefaultUIHandler() {
        return new PermissionUIHandler() {
            @Override
            public void showPermissionExplanation(Activity activity, List<String> permissions, Runnable onConfirm, Runnable onCancel) {
                StringBuilder msg = new StringBuilder("The app needs these permissions:\n\n");
                for (String p : permissions) {
                    msg.append("• ").append(getPermissionDisplayName(p))
                            .append(" - ").append(PERMISSION_EXPLANATIONS.getOrDefault(p, "Required")).append("\n\n");
                }
                new AlertDialog.Builder(activity)
                        .setTitle("Permissions Required")
                        .setMessage(msg.toString())
                        .setPositiveButton("Grant", (d, w) -> onConfirm.run())
                        .setNegativeButton("Cancel", (d, w) -> onCancel.run())
                        .setCancelable(false)
                        .show();
            }

            @Override
            public void showSettingsRedirect(Activity activity, List<String> perms, Runnable onSettings, Runnable onCancel) {
                StringBuilder msg = new StringBuilder("The following permissions are permanently denied:\n\n");
                for (String p : perms) msg.append("• ").append(getPermissionDisplayName(p)).append("\n");
                msg.append("\nPlease enable them in Settings.");
                new AlertDialog.Builder(activity)
                        .setTitle("Permissions Required")
                        .setMessage(msg.toString())
                        .setPositiveButton("Open Settings", (d, w) -> onSettings.run())
                        .setNegativeButton("Cancel", (d, w) -> onCancel.run())
                        .setCancelable(false)
                        .show();
            }

            @Override
            public void showRetryDialog(Activity activity, List<String> perms, Runnable onRetry, Runnable onCancel) {
                StringBuilder msg = new StringBuilder("The following permissions are needed:\n\n");
                for (String p : perms) msg.append("• ").append(getPermissionDisplayName(p)).append("\n");
                msg.append("\nWould you like to retry?");
                new AlertDialog.Builder(activity)
                        .setTitle("Permissions Denied")
                        .setMessage(msg.toString())
                        .setPositiveButton("Retry", (d, w) -> onRetry.run())
                        .setNegativeButton("Cancel", (d, w) -> onCancel.run())
                        .setCancelable(false)
                        .show();
            }
        };
    }
}
