package pt.iris.ui.gallery;

import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;

import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import pt.iris.databinding.FragmentGalleryBinding;


import pt.iris.services.RustService;


public class GalleryFragment extends Fragment {

	private FragmentGalleryBinding binding;
	private Thread test;
	private volatile boolean isRunning = true; // Flag to control the thread

	public View onCreateView(@NonNull LayoutInflater inflater,
							 ViewGroup container, Bundle savedInstanceState) {
		GalleryViewModel galleryViewModel =
				new ViewModelProvider(this).get(GalleryViewModel.class);

		binding = FragmentGalleryBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		final TextView textView = binding.textGallery;
		galleryViewModel.getText().observe(getViewLifecycleOwner(), textView::setText);

		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
		isRunning = false;
	}
}