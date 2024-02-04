package com.example.final_test_wifiaware.ui.home;

import android.annotation.SuppressLint;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;

import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import com.example.final_test_wifiaware.R;
import com.example.final_test_wifiaware.databinding.FragmentHomeBinding;
import com.example.final_test_wifiaware.utils.AppContainer;

public class HomeFragment extends Fragment {
	private FragmentHomeBinding binding;
	private HomeViewModel homeViewModel;

	@SuppressLint("NotifyDataSetChanged")
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		binding = FragmentHomeBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Initialize the ViewModel
		homeViewModel = AppContainer.getInstance().getHomeViewModel();

		// Change UUID Title
		final TextView textView = binding.titleTextView;
		homeViewModel.getMyUUID().observe(getViewLifecycleOwner(), textView::setText);

		// Change UUID Title
		final TextView textView2 = binding.textView2;
		homeViewModel.getPublisherSessionID().observe(getViewLifecycleOwner(), textView2::setText);

		// Change UUID Title
		final TextView textView2Subscribe = binding.textView2Subscribe;
		homeViewModel.getSubscriberSessionID().observe(getViewLifecycleOwner(), textView2Subscribe::setText);
		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
	}
}
