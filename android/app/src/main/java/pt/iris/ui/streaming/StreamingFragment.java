package pt.iris.ui.streaming;

import android.net.Uri;
import android.os.Bundle;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.TextView;
import android.widget.Toast;
import androidx.annotation.NonNull;
import androidx.appcompat.app.AlertDialog;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import pt.iris.R;
import pt.iris.databinding.FragmentStreamingBinding;
import pt.iris.network.lib.WifiAwareModels.DataPathInfo;

import java.util.List;

public class StreamingFragment extends Fragment {

    private FragmentStreamingBinding binding;
    private StreamingViewModel streamingViewModel;

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
        WifiAwareConnectionsAdapter adapter = new WifiAwareConnectionsAdapter();
        binding.clientsRecyclerView.setLayoutManager(new LinearLayoutManager(getContext()));
        binding.clientsRecyclerView.setAdapter(adapter);
        
        // Set up click listeners for connections
        adapter.setOnConnectionClickListener(new WifiAwareConnectionsAdapter.OnConnectionClickListener() {
            @Override
            public void onConnectionClick(DataPathInfo connection) {
                // Handle connection click (e.g., show details)
                showConnectionDetails(connection);
            }
            
            @Override
            public void onConnectionLongClick(DataPathInfo connection) {
                // Handle long click (e.g., disconnect)
                showDisconnectDialog(connection);
            }
            
            @Override
            public void onConnectPeer(DataPathInfo peer) {
                Log.d("StreamingFragment", "onConnectPeer called for peer: " + peer.peerHandle);
                
                // Only allow connections if stream is connected
                Boolean isStreamConnected = streamingViewModel.getIsStreamConnected().getValue();
                Log.d("StreamingFragment", "Stream connected status: " + isStreamConnected);
                
                if (isStreamConnected != null && isStreamConnected) {
                    Log.d("StreamingFragment", "Showing connect dialog");
                    showConnectDialog(peer);
                } else {
                    Log.d("StreamingFragment", "Stream not connected, showing toast");
                    Toast.makeText(getContext(), "Please connect to a stream first before establishing peer connections", Toast.LENGTH_LONG).show();
                }
            }
        });
        
        // Update adapter when network connections change
        streamingViewModel.getWifiAwareConnections().observe(getViewLifecycleOwner(), connections -> {
            adapter.updateConnections(connections);
        });
        
        // Update adapter when session info changes
        streamingViewModel.getWifiAwareSessionInfo().observe(getViewLifecycleOwner(), sessionInfo -> {
            adapter.updateSessionInfo(sessionInfo);
        });
        
        // Update adapter based on stream connection state
        streamingViewModel.getIsStreamConnected().observe(getViewLifecycleOwner(), isConnected -> {
            Log.d("StreamingFragment", "Stream connection state changed: " + isConnected);
            adapter.setConnectionsEnabled(isConnected != null && isConnected);
        });
    }

    private boolean isWifiAwareRelatedLog(String log) {
        // Filter logs to show only WiFi Aware and related logs
        String logLower = log.toLowerCase();
        return logLower.contains("wifiaware") || 
               logLower.contains("wifi aware") ||
               logLower.contains("wifiawarelibrary") ||
               logLower.contains("wifiawaredatapathstmgr") ||
               logLower.contains("wifiawaredatapathstatemanager") ||
               logLower.contains("wifiawarediscoverysession") ||
               logLower.contains("wifiaware:") ||
               logLower.contains("nan") ||
               logLower.contains("networkspecifier") ||
               logLower.contains("data path") ||
               logLower.contains("peer") ||
               logLower.contains("discovery session") ||
               logLower.contains("publishing") ||
               logLower.contains("subscribing") ||
               logLower.contains("connection") ||
               logLower.contains("cluster") ||
               logLower.contains("attach") ||
               logLower.contains("session terminated") ||
               logLower.contains("service discovered") ||
               logLower.contains("service lost") ||
               logLower.contains("role changed") ||
               logLower.contains("error");
    }
    
    private void updateLogsDisplay(TextView logsTextView) {
        List<String> logs = streamingViewModel.getConnectionLogs().getValue();
        if (logs != null && !logs.isEmpty()) {
            StringBuilder logsText = new StringBuilder();
            for (String log : logs) {
                if (isWifiAwareRelatedLog(log)) {
                    logsText.append(log).append("\n");
                }
            }
            logsTextView.setText(logsText.toString());
        } else {
            logsTextView.setText("Connecting to peer...\n");
        }
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

    private void showConnectionDetails(DataPathInfo connection) {
        if (getContext() == null || !isAdded()) {
            Log.w("StreamingFragment", "Cannot show dialog - context is null or fragment not added");
            return;
        }
        
        String connectionStatus = connection.getConnectionStatus();
        String details = "Peer: " + connection.peerHandle.toString() + "\n" +
                        "Status: " + connectionStatus + "\n" +
                        "Address: " + connection.getDisplayAddress();
        
        if (connection.isConnected()) {
            details += "\nDuration: " + connection.getDurationString() + "\n" +
                      "Created: " + new java.text.SimpleDateFormat("HH:mm:ss").format(new java.util.Date(connection.createdTime));
        } else {
            details += "\nDiscovered but not connected";
        }
        
        Log.d("StreamingFragment", "Showing connection details dialog");
        new AlertDialog.Builder(getContext())
                .setTitle("Wi-Fi Aware Peer Details")
                .setMessage(details)
                .setPositiveButton("OK", null)
                .show();
    }
    
    private void showConnectDialog(DataPathInfo peer) {
        if (getContext() == null || !isAdded()) {
            Log.w("StreamingFragment", "Cannot show dialog - context is null or fragment not added");
            return;
        }
        
        Log.d("StreamingFragment", "Creating and showing connect dialog");
        new AlertDialog.Builder(getContext())
                .setTitle("Connect to Wi-Fi Aware Peer")
                .setMessage("Would you like to establish a connection to this discovered peer?")
                .setPositiveButton("Connect", (dialog, which) -> {
                    Log.d("StreamingFragment", "User confirmed connection to peer");
                    streamingViewModel.connectToPeer(peer.peerHandle);
                    Toast.makeText(getContext(), "Attempting to connect to peer...", Toast.LENGTH_SHORT).show();
                })
                .setNegativeButton("Cancel", (dialog, which) -> {
                    Log.d("StreamingFragment", "User cancelled connection");
                })
                .show();
    }
    
    private void showDisconnectDialog(DataPathInfo connection) {
        if (getContext() == null || !isAdded()) {
            Log.w("StreamingFragment", "Cannot show dialog - context is null or fragment not added");
            return;
        }
        
        Log.d("StreamingFragment", "Creating and showing disconnect dialog");
        new AlertDialog.Builder(getContext())
                .setTitle("Disconnect Wi-Fi Aware Connection")
                .setMessage("Are you sure you want to disconnect from this peer?")
                .setPositiveButton("Disconnect", (dialog, which) -> {
                    Log.d("StreamingFragment", "User confirmed disconnection");
                    // TODO: Implement disconnect functionality
                    Toast.makeText(getContext(), "Disconnect functionality not yet implemented", Toast.LENGTH_SHORT).show();
                })
                .setNegativeButton("Cancel", (dialog, which) -> {
                    Log.d("StreamingFragment", "User cancelled disconnection");
                })
                .show();
    }
}
