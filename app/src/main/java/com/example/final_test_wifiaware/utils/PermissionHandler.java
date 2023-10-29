package com.example.final_test_wifiaware.utils;

import android.app.Activity;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.PackageManager.NameNotFoundException;
import android.content.Context;
import androidx.core.app.ActivityCompat;
import androidx.core.content.ContextCompat;
import java.util.ArrayList;
import java.util.List;

public class PermissionHandler {

	private static final int MY_PERMISSIONS_REQUEST_CODE = 123; // You can change this to your desired request code.

	// Function to request all permissions from the manifest
	public static void requestAppPermissions(Context context) {
		try {
			PackageManager packageManager = context.getPackageManager();
			String packageName = context.getPackageName();
			PackageInfo packageInfo = packageManager.getPackageInfo(packageName, PackageManager.GET_PERMISSIONS);
			String[] permissions = packageInfo.requestedPermissions;

			if (permissions != null) {
				List<String> permissionsToRequest = new ArrayList<>();
				for (String permission : permissions) {
					if (ContextCompat.checkSelfPermission(context, permission) != PackageManager.PERMISSION_GRANTED) {
						permissionsToRequest.add(permission);
					}
				}

				if (!permissionsToRequest.isEmpty()) {
					ActivityCompat.requestPermissions((Activity) context, permissionsToRequest.toArray(new String[0]), MY_PERMISSIONS_REQUEST_CODE);
				}
			}
		} catch (NameNotFoundException e) {
			e.printStackTrace();
		}
	}
}
