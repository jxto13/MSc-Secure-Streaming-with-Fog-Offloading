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
	private HomeViewModel homeViewModel;
	private CustomAdapter adapter;
	private RecyclerView recyclerView;

	@SuppressLint("NotifyDataSetChanged")
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		binding = FragmentHomeBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Initialize the ViewModel
		homeViewModel = AppContainer.getInstance().getHomeViewModel();

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

		final Switch switchPublisher = binding.switch1;
		final Switch switchSubscriber = binding.switch2;
		//final Switch pingAll = binding.switch3;

		// Initialize RecyclerView and adapter
		recyclerView = root.findViewById(R.id.recyclerView);
		recyclerView.setLayoutManager(new LinearLayoutManager(requireContext()));

		// Create an instance of the custom adapter
		adapter = new CustomAdapter(requireContext(), homeViewModel.getConnectionsList());

		// Set the adapter for the ListView
		recyclerView.setAdapter(adapter);

		// Observe the connectionsLiveData from the ViewModel
		homeViewModel.getConnectionsLiveData().observe(getViewLifecycleOwner(), connections -> {
			adapter.updateData(connections);
			// Update your UI with the new connections list
			adapter.notifyDataSetChanged();
		});



		AppContainer.getInstance().setSwitchPublisher(switchPublisher);
		AppContainer.getInstance().setSwitchSubscribe(switchSubscriber);
		//AppContainer.getInstance().setPingSwitch(pingAll);

		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
	}
}
