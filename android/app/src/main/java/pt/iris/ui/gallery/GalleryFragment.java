package pt.iris.ui.gallery;

import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;

import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import pt.iris.databinding.FragmentGalleryBinding;

public class GalleryFragment extends Fragment {

	private FragmentGalleryBinding binding;

	@Override
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		GalleryViewModel slideshowViewModel = new ViewModelProvider(this).get(GalleryViewModel.class);

		binding = FragmentGalleryBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Find the Button using the binding
		Button openMpvButton = binding.openMpvButton;

		// Set an OnClickListener for the button
		openMpvButton.setOnClickListener(v -> openMpv());

		return root;
	}

	private void openMpv() {
		// Create an intent to open the URL with mpv
		Intent launchIntent = requireActivity().getPackageManager().getLaunchIntentForPackage("is.xyz.mpv");
		if (launchIntent != null) {
			startActivity(launchIntent);//null pointer check in case package name was not found
		}
		/*Intent intent = new Intent(Intent.ACTION_VIEW);
		intent.setDataAndType(Uri.parse("udp://127.0.0.1:12346"), "video/*");

		// Specify the package for MPV Player if needed
		intent.setPackage("is.xyz.mpv");

		// Check if there's an activity that can handle this intent
		if (intent.resolveActivity(requireActivity().getPackageManager()) != null) {
			startActivity(intent);
		} else {
			// Handle the case where MPV is not installed or cannot handle the intent
			// For example, you could show a message to the user
			System.out.println("MPV Player is not installed or cannot handle this URL.");
		}*/
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
	}
}
