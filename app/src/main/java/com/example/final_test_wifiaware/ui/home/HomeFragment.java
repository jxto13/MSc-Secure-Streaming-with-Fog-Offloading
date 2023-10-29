package com.example.final_test_wifiaware.ui.home;


import android.os.Bundle;

import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;
import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;
import com.example.final_test_wifiaware.MainActivity;
import com.example.final_test_wifiaware.databinding.FragmentHomeBinding;
import com.example.final_test_wifiaware.network.NetworkInitializer;


public class HomeFragment extends Fragment {
	NetworkInitializer wifiAwareNetwork;
	private FragmentHomeBinding binding;


	public View onCreateView(@NonNull LayoutInflater inflater,
							 ViewGroup container, Bundle savedInstanceState) {
		binding = FragmentHomeBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		final TextView textView = binding.textHome;
		HomeViewModel homeViewModel = new ViewModelProvider(this).get(HomeViewModel.class);

		// Observe the LiveData for text and set it to the TextView.
		homeViewModel.getText().observe(getViewLifecycleOwner(), textView::setText);


		// Access the wifiAwareNetwork object from the MainActivity
		if (getActivity() instanceof MainActivity) {
			wifiAwareNetwork = ((MainActivity) getActivity()).wifiAwareNetwork;
		}

		return root;
	}


	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
	}
}
