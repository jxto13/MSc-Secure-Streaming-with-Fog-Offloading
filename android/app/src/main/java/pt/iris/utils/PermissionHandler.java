package pt.iris.utils;

import android.Manifest;
import android.app.Activity;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.provider.Settings;
import android.os.Build;
import androidx.annotation.NonNull;
import androidx.appcompat.app.AlertDialog;
import androidx.core.app.ActivityCompat;
import androidx.core.content.ContextCompat;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Enhanced Permission Manager with UI for WiFi Aware Video Streaming App
 * Handles all required permissions with user-friendly dialogs and explanations
 */
public class PermissionHandler {

    private static final int PERMISSION_REQUEST_CODE = 1001;
    private static final int SETTINGS_REQUEST_CODE = 1002;

    // Required permissions for WiFi Aware functionality
    private static final String[] REQUIRED_PERMISSIONS = {
            Manifest.permission.ACCESS_WIFI_STATE,
            Manifest.permission.CHANGE_WIFI_STATE,
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.ACCESS_COARSE_LOCATION,
            Manifest.permission.NEARBY_WIFI_DEVICES, // Android 13+
            Manifest.permission.INTERNET,
            Manifest.permission.ACCESS_NETWORK_STATE,
            Manifest.permission.CHANGE_NETWORK_STATE,
            Manifest.permission.WRITE_EXTERNAL_STORAGE, // For logging if needed (Android 12 and below)
            Manifest.permission.READ_EXTERNAL_STORAGE, // For reading files (Android 12 and below)
            "android.permission.READ_MEDIA_IMAGES", // Android 13+ - using string to avoid compilation issues
            "android.permission.READ_MEDIA_VIDEO", // Android 13+ - using string to avoid compilation issues
            "android.permission.READ_MEDIA_AUDIO"  // Android 13+ - using string to avoid compilation issues
    };

    // Permission explanations for users
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
        put("android.permission.READ_MEDIA_IMAGES", "Required for accessing images for streaming (Android 13+)");
        put("android.permission.READ_MEDIA_VIDEO", "Required for accessing video files for streaming (Android 13+)");
        put("android.permission.READ_MEDIA_AUDIO", "Required for accessing audio files for streaming (Android 13+)");
    }};

    public interface PermissionCallback {
        void onPermissionsGranted();
        void onPermissionsDenied(List<String> deniedPermissions);
        void onPermissionsPermanentlyDenied(List<String> permanentlyDeniedPermissions);
    }

    private static PermissionCallback currentCallback;

    /**
     * Request all required permissions with user-friendly UI
     */
    public static void requestAppPermissions(Activity activity, PermissionCallback callback) {
        currentCallback = callback;
        
        List<String> missingPermissions = getMissingPermissions(activity);
        
        if (missingPermissions.isEmpty()) {
            // All permissions already granted
            if (callback != null) {
                callback.onPermissionsGranted();
            }
            return;
        }

        // Show explanation dialog before requesting permissions
        showPermissionExplanationDialog(activity, missingPermissions);
    }

    /**
     * Legacy method for backward compatibility
     */
    public static void requestAppPermissions(Context context) {
        if (context instanceof Activity) {
            requestAppPermissions((Activity) context, null);
        }
    }

    /**
     * Get list of missing permissions
     */
    private static List<String> getMissingPermissions(Context context) {
        List<String> missingPermissions = new ArrayList<>();
        
        for (String permission : REQUIRED_PERMISSIONS) {
            // Skip NEARBY_WIFI_DEVICES for Android < 13
            if (permission.equals(Manifest.permission.NEARBY_WIFI_DEVICES) && 
                Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
                continue;
            }
            
            // Skip legacy storage permissions for Android 13+
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                if (permission.equals(Manifest.permission.READ_EXTERNAL_STORAGE) ||
                    permission.equals(Manifest.permission.WRITE_EXTERNAL_STORAGE)) {
                    continue;
                }
            }
            
            // Skip new media permissions for Android < 13
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
                if (permission.equals("android.permission.READ_MEDIA_IMAGES") ||
                    permission.equals("android.permission.READ_MEDIA_VIDEO") ||
                    permission.equals("android.permission.READ_MEDIA_AUDIO")) {
                    continue;
                }
            }
            
            if (ContextCompat.checkSelfPermission(context, permission) != PackageManager.PERMISSION_GRANTED) {
                missingPermissions.add(permission);
            }
        }
        
        return missingPermissions;
    }

    /**
     * Show explanation dialog for required permissions
     */
    private static void showPermissionExplanationDialog(Activity activity, List<String> permissions) {
        StringBuilder message = new StringBuilder();
        message.append("This WiFi Aware Video Streaming app requires the following permissions to function properly:\n\n");
        
        for (String permission : permissions) {
            String explanation = PERMISSION_EXPLANATIONS.get(permission);
            if (explanation != null) {
                String permissionName = getPermissionDisplayName(permission);
                message.append("• ").append(permissionName).append(":\n  ").append(explanation).append("\n\n");
            }
        }
        
        message.append("Would you like to grant these permissions?");

        new AlertDialog.Builder(activity)
                .setTitle("Permissions Required")
                .setMessage(message.toString())
                .setIcon(android.R.drawable.ic_dialog_info)
                .setPositiveButton("Grant Permissions", (dialog, which) -> {
                    requestPermissions(activity, permissions);
                })
                .setNegativeButton("Cancel", (dialog, which) -> {
                    if (currentCallback != null) {
                        currentCallback.onPermissionsDenied(permissions);
                    }
                })
                .setCancelable(false)
                .show();
    }

    /**
     * Request the actual permissions
     */
    private static void requestPermissions(Activity activity, List<String> permissions) {
        ActivityCompat.requestPermissions(
                activity, 
                permissions.toArray(new String[0]), 
                PERMISSION_REQUEST_CODE
        );
    }

    /**
     * Handle permission request results
     */
    public static void onRequestPermissionsResult(Activity activity, int requestCode, 
                                                 @NonNull String[] permissions, 
                                                 @NonNull int[] grantResults) {
        if (requestCode != PERMISSION_REQUEST_CODE) {
            return;
        }

        List<String> deniedPermissions = new ArrayList<>();
        List<String> permanentlyDeniedPermissions = new ArrayList<>();

        for (int i = 0; i < permissions.length; i++) {
            if (grantResults[i] != PackageManager.PERMISSION_GRANTED) {
                if (ActivityCompat.shouldShowRequestPermissionRationale(activity, permissions[i])) {
                    deniedPermissions.add(permissions[i]);
                } else {
                    permanentlyDeniedPermissions.add(permissions[i]);
                }
            }
        }

        if (deniedPermissions.isEmpty() && permanentlyDeniedPermissions.isEmpty()) {
            // All permissions granted
            if (currentCallback != null) {
                currentCallback.onPermissionsGranted();
            }
        } else if (!permanentlyDeniedPermissions.isEmpty()) {
            // Some permissions permanently denied
            showSettingsDialog(activity, permanentlyDeniedPermissions);
        } else {
            // Some permissions denied but can be requested again
            showPermissionDeniedDialog(activity, deniedPermissions);
        }
    }

    /**
     * Show dialog for permanently denied permissions
     */
    private static void showSettingsDialog(Activity activity, List<String> permanentlyDeniedPermissions) {
        StringBuilder message = new StringBuilder();
        message.append("The following permissions were permanently denied and are required for the app to function:\n\n");
        
        for (String permission : permanentlyDeniedPermissions) {
            String permissionName = getPermissionDisplayName(permission);
            message.append("• ").append(permissionName).append("\n");
        }
        
        message.append("\nPlease enable them in Settings > Apps > Iris > Permissions");

        new AlertDialog.Builder(activity)
                .setTitle("Permissions Required")
                .setMessage(message.toString())
                .setIcon(android.R.drawable.ic_dialog_alert)
                .setPositiveButton("Open Settings", (dialog, which) -> {
                    Intent intent = new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS);
                    Uri uri = Uri.fromParts("package", activity.getPackageName(), null);
                    intent.setData(uri);
                    activity.startActivityForResult(intent, SETTINGS_REQUEST_CODE);
                })
                .setNegativeButton("Cancel", (dialog, which) -> {
                    if (currentCallback != null) {
                        currentCallback.onPermissionsPermanentlyDenied(permanentlyDeniedPermissions);
                    }
                })
                .setCancelable(false)
                .show();
    }

    /**
     * Show dialog for denied permissions that can be requested again
     */
    private static void showPermissionDeniedDialog(Activity activity, List<String> deniedPermissions) {
        StringBuilder message = new StringBuilder();
        message.append("The following permissions are required for WiFi Aware functionality:\n\n");
        
        for (String permission : deniedPermissions) {
            String permissionName = getPermissionDisplayName(permission);
            String explanation = PERMISSION_EXPLANATIONS.get(permission);
            message.append("• ").append(permissionName);
            if (explanation != null) {
                message.append(":\n  ").append(explanation);
            }
            message.append("\n\n");
        }
        
        message.append("Would you like to try again?");

        new AlertDialog.Builder(activity)
                .setTitle("Permissions Denied")
                .setMessage(message.toString())
                .setIcon(android.R.drawable.ic_dialog_alert)
                .setPositiveButton("Try Again", (dialog, which) -> {
                    requestPermissions(activity, deniedPermissions);
                })
                .setNegativeButton("Cancel", (dialog, which) -> {
                    if (currentCallback != null) {
                        currentCallback.onPermissionsDenied(deniedPermissions);
                    }
                })
                .setCancelable(false)
                .show();
    }

    /**
     * Get user-friendly permission name
     */
    private static String getPermissionDisplayName(String permission) {
        switch (permission) {
            case Manifest.permission.ACCESS_FINE_LOCATION:
                return "Precise Location";
            case Manifest.permission.ACCESS_COARSE_LOCATION:
                return "Approximate Location";
            case Manifest.permission.ACCESS_WIFI_STATE:
                return "WiFi State Access";
            case Manifest.permission.CHANGE_WIFI_STATE:
                return "WiFi State Changes";
            case Manifest.permission.NEARBY_WIFI_DEVICES:
                return "Nearby WiFi Devices";
            case Manifest.permission.INTERNET:
                return "Internet Access";
            case Manifest.permission.ACCESS_NETWORK_STATE:
                return "Network State Access";
            case Manifest.permission.CHANGE_NETWORK_STATE:
                return "Network State Changes";
            case Manifest.permission.WRITE_EXTERNAL_STORAGE:
                return "Storage Write Access";
            case Manifest.permission.READ_EXTERNAL_STORAGE:
                return "Storage Read Access";
            default:
                return permission.substring(permission.lastIndexOf('.') + 1);
        }
    }

    /**
     * Check if all required permissions are granted
     */
    public static boolean areAllPermissionsGranted(Context context) {
        return getMissingPermissions(context).isEmpty();
    }

    /**
     * Get list of missing permission names for display
     */
    public static List<String> getMissingPermissionNames(Context context) {
        List<String> missingPermissions = getMissingPermissions(context);
        List<String> missingNames = new ArrayList<>();
        
        for (String permission : missingPermissions) {
            missingNames.add(getPermissionDisplayName(permission));
        }
        
        return missingNames;
    }
}
