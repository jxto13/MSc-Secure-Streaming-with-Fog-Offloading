package com.example.final_test_wifiaware.ui.home;

import static android.content.Context.WIFI_SERVICE;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;
import android.net.wifi.aware.AttachCallback;
import android.net.wifi.aware.DiscoverySessionCallback;
import android.net.wifi.aware.PeerHandle;
import android.net.wifi.aware.PublishConfig;
import android.net.wifi.aware.PublishDiscoverySession;
import android.net.wifi.aware.SubscribeConfig;
import android.net.wifi.aware.SubscribeDiscoverySession;
import android.net.wifi.aware.WifiAwareManager;
import android.net.wifi.aware.WifiAwareSession;
import android.os.Bundle;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.TextView;

import androidx.annotation.NonNull;
import androidx.core.app.ActivityCompat;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import com.example.final_test_wifiaware.R;
import com.example.final_test_wifiaware.databinding.FragmentHomeBinding;

import java.util.List;
import java.util.Objects;
import java.util.UUID;

public class HomeFragment extends Fragment {

	private FragmentHomeBinding binding;
	private HomeViewModel homeViewModel;

	private WifiAwareSession mWifiAwareSession;

	public void startDiscovery() {
		Context context = this.getContext();
		// Initialize Wi-Fi Aware manager
		assert context != null;
		WifiAwareManager wifiAwareManager =
				(WifiAwareManager) context.getSystemService(Context.WIFI_AWARE_SERVICE);


		// Check if Wi-Fi Aware is supported on this device
		if (wifiAwareManager.isAvailable()) {
			Log.d("wifiAwareManager", "available");
			// Create a callback for session events
			DiscoverySessionCallback sessionCallback = new DiscoverySessionCallback() {
				@Override
				public void onServiceDiscovered(PeerHandle peerHandle, byte[] serviceSpecificInfo, List<byte[]> matchFilter) {
					// Handle discovered services here
					String serviceName = new String(serviceSpecificInfo);

					Log.d("ServiceDiscovered", "Service name: " + serviceName + " | peerHandle: " + peerHandle);
					// Subscribe to the service here using the known service name or any criteria you require.
				}
				@Override
				public void onPublishStarted(PublishDiscoverySession session) {
					Log.d("onPublishStarted", session.toString());

					// Publish session started, you can now send data
				}

				@Override
				public void onSubscribeStarted(SubscribeDiscoverySession session) {
					// Subscribe session started, you can now receive data
					Log.d("onSubscribeStarted", session.toString());
					//session.sendMessage(1,"send from pixel 3a xl".getBytes());
				}

				@Override
				public void onSessionConfigFailed() {
					Log.e("SessionConfigFailed", "Session configuration failed");
					// Session configuration failed
				}
			};
			String uuid = UUID.randomUUID().toString();
			// Create a SubscribeConfig for subscribing to data
			PublishConfig publishConfig = new PublishConfig.Builder()
					.setServiceSpecificInfo("Stream Stream".getBytes())
					.setServiceName("test") // Set the same service name as in publish
					.build();

			// Create a SubscribeConfig for subscribing to data
			SubscribeConfig subscribeConfig = new SubscribeConfig.Builder()
					.setServiceName("test")
					.build();

			// Attach to Wi-Fi Aware and create a session
			wifiAwareManager.attach(new AttachCallback() {
				@Override
				public void onAttached(WifiAwareSession session) {
					mWifiAwareSession = session;

					// Create a publish discovery session
					if (ActivityCompat.checkSelfPermission(context, Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
						// TODO: Consider calling
						//    ActivityCompat#requestPermissions
						// here to request the missing permissions, and then overriding
						//   public void onRequestPermissionsResult(int requestCode, String[] permissions,
						//                                          int[] grantResults)
						// to handle the case where the user grants the permission. See the documentation
						// for ActivityCompat#requestPermissions for more details.
						Log.d("requestPermissions", "ActivityCompat#requestPermissions return 0");

						return;
					}

					mWifiAwareSession.publish(publishConfig, sessionCallback, null);

					// Create a subscribe discovery session
					mWifiAwareSession.subscribe(subscribeConfig, sessionCallback, null);
				}

				@Override
				public void onAttachFailed() {
					// Wi-Fi Aware attachment failed
				}
			}, null);
		} else {
			Log.d("wifiAwareManager", "not available");
			// Wi-Fi Aware is not available on this device
		}

}


	public void stopDiscovery() {
		if (mWifiAwareSession != null) {
			// Close the Wi-Fi Aware session to terminate publish and subscribe sessions
			mWifiAwareSession.close();
		}
	}

	private void refreshDiscovery() {
		stopDiscovery(); // Stops the discovery session
		startDiscovery(); // Start a new discovery session
	}

	public View onCreateView(@NonNull LayoutInflater inflater,
							 ViewGroup container, Bundle savedInstanceState) {
		binding = FragmentHomeBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		final TextView textView = binding.textHome;
		homeViewModel = new ViewModelProvider(this).get(HomeViewModel.class);

		// Observe the LiveData for text and set it to the TextView.
		homeViewModel.getText().observe(getViewLifecycleOwner(), textView::setText);

		Button refreshButton = root.findViewById(R.id.refreshDiscoveryButton);
		refreshButton.setOnClickListener(new View.OnClickListener() {
			@Override
			public void onClick(View v) {
				// When the refresh button is clicked, refresh the discovery session.
				refreshDiscovery();
			}
		});


		// Button to start the Wi-Fi Aware service and discovery.
//		Button startDiscoveryButton = binding.startDiscoveryButton;
//		startDiscoveryButton.setOnClickListener(v -> {
//			// Start the Wi-Fi Aware discovery.
//			homeViewModel.startWifiAwareDiscovery();
//		});
//
//		Button stopDiscoveryButton = binding.stopDiscoveryButton;
//		stopDiscoveryButton.setOnClickListener(v -> {
//			// Stop the discovery in the ViewModel.
//			homeViewModel.stopDiscovery();
//		});

		// Observe the LiveData for discovered devices and update the UI accordingly.
		homeViewModel.getDiscoveredDevices().observe(getViewLifecycleOwner(), deviceList -> {
			// Update the UI to display the list of discovered devices.
			// You can use a RecyclerView or other UI elements to show the devices.
		});

		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		stopDiscovery();
		binding = null;
	}
}
