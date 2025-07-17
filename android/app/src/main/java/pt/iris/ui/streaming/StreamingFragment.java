package pt.iris.ui.streaming;

import android.net.Uri;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Toast;
import androidx.annotation.NonNull;
import androidx.appcompat.app.AlertDialog;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import pt.iris.databinding.FragmentStreamingBinding;

public class StreamingFragment extends Fragment {

    private FragmentStreamingBinding binding;
    private StreamingViewModel streamingViewModel;
    private WifiAwareConnectionsAdapter connectionsAdapter;

    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
        streamingViewModel = new ViewModelProvider(this).get(StreamingViewModel.class);

        binding = FragmentStreamingBinding.inflate(inflater, container, false);
        View root = binding.getRoot();

        setupWifiAwareConnectionsRecyclerView();
        setupUI();
        observeViewModel();

        return root;
    }

    private void setupPlayerView() {
        // Set player to the view only if it exists
        if (streamingViewModel.getPlayer() != null) {
            binding.playerView.setPlayer(streamingViewModel.getPlayer());
        }
    }

    private void setupWifiAwareConnectionsRecyclerView() {
        connectionsAdapter = new WifiAwareConnectionsAdapter();
        binding.clientsRecyclerView.setLayoutManager(new LinearLayoutManager(getContext()));
        binding.clientsRecyclerView.setAdapter(connectionsAdapter);

        // Set up item click listener for connection details
        connectionsAdapter.setOnConnectionClickListener(connection -> {
            new AlertDialog.Builder(requireContext())
                    .setTitle("WiFi Aware Connection Details")
                    .setMessage("Peer: " + connection.peerHandleStr + "\n" +
                              "IPv6: " + connection.ipAddress + "\n" +
                              "Duration: " + connection.createdTime + "\n" +
                              "Status: " + (connection.isActive ? "Active" : "Inactive") + "\n" +
                              "My Role: " + connection.myRole + "\n" +
                              "Peer Role: " + connection.peerRole)
                    .setPositiveButton("OK", null)
                    .show();
        });
    }

    private void setupUI() {
        // Stream URI connection controls
        binding.btnConnectStream.setOnClickListener(v -> {
            String uriText = binding.editStreamUri.getText().toString().trim();
            if (uriText.isEmpty()) {
                Toast.makeText(getContext(), "Please enter a stream URI", Toast.LENGTH_SHORT).show();
                return;
            }
            
            try {
                Uri streamUri = Uri.parse(uriText);
                streamingViewModel.connectToStream(requireContext(), streamUri);
                setupPlayerView(); // Set up the player view now that player exists
                updateStreamButtons(true);
                Toast.makeText(getContext(), "Connecting to stream...", Toast.LENGTH_SHORT).show();
            } catch (Exception e) {
                Toast.makeText(getContext(), "Failed to connect to stream", Toast.LENGTH_SHORT).show();
            }
        });

        binding.btnDisconnectStream.setOnClickListener(v -> {
            streamingViewModel.disconnectStream();
            binding.playerView.setPlayer(null); // Remove player from view
            updateStreamButtons(false);

            
            Toast.makeText(getContext(), "Stream disconnected and cleaned up", Toast.LENGTH_SHORT).show();
        });

        // Hide the add client button since we get connections automatically from NetworkInitializer
        if (binding.addClientButton != null) {
            binding.addClientButton.setVisibility(View.GONE);
        }
    }

    private void updateStreamButtons(boolean connected) {
        binding.btnConnectStream.setEnabled(!connected);
        binding.btnDisconnectStream.setEnabled(connected);
        binding.editStreamUri.setEnabled(!connected);
    }

    private void observeViewModel() {
        // Observe real WiFi Aware connections from NetworkInitializer
        streamingViewModel.getWifiAwareConnections().observe(getViewLifecycleOwner(), connections -> {
            connectionsAdapter.updateConnections(connections);
            binding.clientCountText.setText(String.format("WiFi Aware Connections: %d", connections.size()));
        });

        // Observe network stats
        streamingViewModel.getNetworkStats().observe(getViewLifecycleOwner(), stats -> {
            binding.totalBytesText.setText(String.format("Total: %.2f MB", stats.totalBytesReceived / 1024.0 / 1024.0));
            binding.bitrateText.setText(String.format("Bitrate: %.1f kbps", stats.currentBitrate / 1000.0));
            binding.packetLossText.setText(String.format("Loss: %.2f%%", stats.packetLossRate));
            
            // Display WiFi Aware specific data path stats in the stats section
            if (stats.wifiAwareDataPaths != null && !stats.wifiAwareDataPaths.equals("N/A")) {
                binding.dataPathStatsText.setText("NAN Paths: " + stats.wifiAwareDataPaths);
                binding.nanAvailableText.setText("NAN Available: Yes");
            } else {
                binding.dataPathStatsText.setText("NAN Paths: N/A");
                binding.nanAvailableText.setText("NAN Available: No");
            }
        });

        // Observe player ready state
        streamingViewModel.getIsPlayerReady().observe(getViewLifecycleOwner(), isReady -> {
            binding.playerStatusText.setText(isReady ? "Status: Playing" : "Status: Loading...");
            binding.playerStatusText.setTextColor(getResources().getColor(
                    isReady ? android.R.color.holo_green_dark : android.R.color.holo_orange_dark, 
                    null));
        });

        // Observe stream connection state
        streamingViewModel.getIsStreamConnected().observe(getViewLifecycleOwner(), connected -> {
            updateStreamButtons(connected);
            if (connected) {
                binding.playerStatusText.setText("Status: Stream Connected");
            }
        });

    }

    @Override
    public void onResume() {
        super.onResume();
        if (streamingViewModel.getPlayer() != null) {
            streamingViewModel.getPlayer().play();
        }
        // Refresh connections when fragment becomes visible
        streamingViewModel.forceRefreshConnections();
    }

    @Override
    public void onPause() {
        super.onPause();
        if (streamingViewModel.getPlayer() != null) {
            streamingViewModel.getPlayer().pause();
        }
    }

    @Override
    public void onDestroyView() {
        super.onDestroyView();
        binding = null;
    }
}
