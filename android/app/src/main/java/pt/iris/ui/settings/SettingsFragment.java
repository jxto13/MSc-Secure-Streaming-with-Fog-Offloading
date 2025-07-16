package pt.iris.ui.settings;

import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import pt.iris.MainActivity;
import pt.iris.databinding.FragmentSettingsBinding;

public class SettingsFragment extends Fragment {

    private FragmentSettingsBinding binding;
    private SettingsViewModel settingsViewModel;
    private Handler statusUpdateHandler;
    private Runnable statusUpdateRunnable;

    @Override
    public View onCreateView(@NonNull LayoutInflater inflater,
                             ViewGroup container, Bundle savedInstanceState) {
        binding = FragmentSettingsBinding.inflate(inflater, container, false);
        return binding.getRoot();
    }

    @Override
    public void onViewCreated(@NonNull View view, @Nullable Bundle savedInstanceState) {
        super.onViewCreated(view, savedInstanceState);
        
        MainActivity activity = (MainActivity) getActivity();
        if (activity == null) return;
        
        settingsViewModel = new ViewModelProvider(this, 
            activity.getAppContainer().getSettingsViewModelFactory())
            .get(SettingsViewModel.class);
        
        // Initialize ViewModel with context
        settingsViewModel.initializeWithContext(requireContext());
        
        setupObservers();
        setupClickListeners();
        setupStatusUpdates();
    }

    private void setupObservers() {
        settingsViewModel.getWifiAwareEnabled().observe(getViewLifecycleOwner(), enabled -> 
            binding.switchWifiAwareEnabled.setChecked(enabled));
        
        settingsViewModel.getNanStatus().observe(getViewLifecycleOwner(), status -> 
            binding.textNanStatus.setText(status));
        
        settingsViewModel.getNanActivePaths().observe(getViewLifecycleOwner(), paths -> 
            binding.textNanActivePaths.setText(paths));
    }

    private void setupClickListeners() {
        binding.switchWifiAwareEnabled.setOnCheckedChangeListener((buttonView, isChecked) -> {
            settingsViewModel.setWifiAwareEnabled(isChecked);
            if (!isChecked) {
                Toast.makeText(getContext(), 
                    "WiFi Aware disabled. Restart the app to apply changes.", 
                    Toast.LENGTH_LONG).show();
            } else {
                Toast.makeText(getContext(), 
                    "WiFi Aware enabled. Restart the app to apply changes.", 
                    Toast.LENGTH_LONG).show();
            }
        });
    }

    private void setupStatusUpdates() {
        statusUpdateHandler = new Handler(Looper.getMainLooper());
        statusUpdateRunnable = new Runnable() {
            @Override
            public void run() {
                if (isAdded() && settingsViewModel != null) {
                    settingsViewModel.updateNanStatus();
                    statusUpdateHandler.postDelayed(this, 2000); // Update every 2 seconds
                }
            }
        };
        statusUpdateHandler.post(statusUpdateRunnable);
    }

    @Override
    public void onDestroyView() {
        super.onDestroyView();
        if (statusUpdateHandler != null && statusUpdateRunnable != null) {
            statusUpdateHandler.removeCallbacks(statusUpdateRunnable);
        }
        binding = null;
    }
}
