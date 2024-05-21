package pt.iris.ui.home;

import android.annotation.SuppressLint;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;

import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;

import pt.iris.databinding.FragmentHomeBinding;
import pt.iris.utils.AppContainer;

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
