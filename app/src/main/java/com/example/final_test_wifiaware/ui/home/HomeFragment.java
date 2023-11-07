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
import com.example.final_test_wifiaware.network.utils.Connection;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.ArrayList;
import java.util.List;

public class HomeFragment extends Fragment {
	private FragmentHomeBinding binding;
	private RecyclerView recyclerView;
	private HomeViewModel homeViewModel;
	private CustomAdapter adapter;

	@SuppressLint("NotifyDataSetChanged")
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		binding = FragmentHomeBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Initialize RecyclerView and adapter
		recyclerView = root.findViewById(R.id.recyclerView);
		recyclerView.setLayoutManager(new LinearLayoutManager(requireContext()));

		// Initialize the ViewModel
		homeViewModel = AppContainer.getInstance().getHomeViewModel();

		// Change UUID Title
		final TextView textView = binding.titleTextView;
		homeViewModel.getMyUUID().observe(getViewLifecycleOwner(), textView::setText);

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

		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
	}
}
