package com.example.final_test_wifiaware.ui.home;

import android.annotation.SuppressLint;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Switch;
import android.widget.TextView;

import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import com.example.final_test_wifiaware.R;
import com.example.final_test_wifiaware.databinding.FragmentHomeBinding;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.ArrayList;
import java.util.List;

public class HomeFragment extends Fragment {
	private FragmentHomeBinding binding;

	@SuppressLint("NotifyDataSetChanged")
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		binding = FragmentHomeBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Initialize the ViewModel
		HomeViewModel homeViewModel = AppContainer.getInstance().getHomeViewModel();

		final TextView textView = binding.titleTextView;
		homeViewModel.getMyUUID().observe(getViewLifecycleOwner(), textView::setText);

		final TextView textView2 = binding.textView2;
		homeViewModel.getPublisherSessionID().observe(getViewLifecycleOwner(), textView2::setText);

		final TextView textView2Subscribe = binding.textView2Subscribe;
		homeViewModel.getSubscriberSessionID().observe(getViewLifecycleOwner(), textView2Subscribe::setText);

		final TextView textVIewClusterID = binding.textClusterID;
		homeViewModel.getClusterID().observe(getViewLifecycleOwner(), textVIewClusterID::setText);

		final TextView textDataPathsCount = binding.textDataPathsCountValue;
		homeViewModel.getDataPaths().observe(getViewLifecycleOwner(), textDataPathsCount::setText);

		final TextView textPublishSessionsCountValue = binding.textPublishSessionsCountValue;
		homeViewModel.getSessionPub().observe(getViewLifecycleOwner(), textPublishSessionsCountValue::setText);

		final TextView textSubscribeSessionsCountValue = binding.textSubscribeSessionsCountValue;
		homeViewModel.getSessionSub().observe(getViewLifecycleOwner(), textSubscribeSessionsCountValue::setText);

		homeViewModel.setSwitchPublisher(binding.switch1);
		homeViewModel.setSwitchSubscribe(binding.switch2);

		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
	}
}
