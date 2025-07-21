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

import java.net.Inet6Address;
import java.net.InetAddress;
import java.net.NetworkInterface;
import java.net.SocketException;
import java.util.Enumeration;
import java.util.List;
import java.util.Map;

public class StreamingFragment extends Fragment {

    private FragmentStreamingBinding binding;
    private StreamingViewModel streamingViewModel;

    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
        // Use Activity scope to ensure the same ViewModel instance is shared
        streamingViewModel = new ViewModelProvider(requireActivity()).get(StreamingViewModel.class);

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
            public void onConnectPeer(DataPathInfo peer) {
                // Only allow connect if peer is publisher-discovered (not subscriber)
                // If DataPathInfo or peer has a flag/role, check it here. Otherwise, always allow (UI will only show publisher peers)
                Log.d("StreamingFragment", "onConnectPeer called for peer: " + peer.peerHandle);
                if (getContext() == null || !isAdded()) return;
                new AlertDialog.Builder(getContext())
                        .setTitle("Connect to Peer")
                        .setMessage("Do you want to connect to this peer?\n" + peer.peerHandle)
                        .setPositiveButton("Connect", (dialog, which) -> {
                            streamingViewModel.connectToPeer(peer.peerHandle);
                            Toast.makeText(getContext(), "Connecting to peer...", Toast.LENGTH_SHORT).show();
                        })
                        .setNegativeButton("Cancel", null)
                        .show();
            }
        });
        
        // Update adapter when network connections change
        streamingViewModel.getWifiAwareConnections().observe(getViewLifecycleOwner(), connections -> {
            int size = (connections != null) ? connections.size() : 0;
            Log.d("mine", "[StreamingFragment] LiveData observer triggered. Map size: " + size);
            if (connections != null) {
                for (Map.Entry<?, ?> entry : connections.entrySet()) {
                    Log.d("mine", "[StreamingFragment] LiveData entry: " + entry.getKey() + " -> " + entry.getValue());
                }
            }
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
        
        // Make sure the ViewModel releases resources
        if (streamingViewModel != null) {
            streamingViewModel.stopStreaming();
        }
        
        binding = null;
    }
    
    @Override
    public void onDestroy() {
        super.onDestroy();
        
        // Final cleanup to ensure Rust client is stopped
        try {
            pt.iris.services.RustService.stopClient();
        } catch (Exception e) {
            Log.w("StreamingFragment", "Error stopping Rust client in onDestroy", e);
        }
    }

    private void showConnectionDetails(DataPathInfo connection) {
        if (getContext() == null || !isAdded()) {
            Log.w("StreamingFragment", "Cannot show details - context is null or fragment not added");
            return;
        }

        showIPv6InterfaceInfoDialog(connection.getInterfaceString());
    }

    private void showIPv6InterfaceInfoDialog(String interfaceName) {
        if (getContext() == null || !isAdded()) {
            Log.w("StreamingFragment", "Cannot show details - context is null or fragment not added");
            return;
        }

        StringBuilder details = new StringBuilder();

        try {
            NetworkInterface networkInterface = NetworkInterface.getByName(interfaceName);
            if (networkInterface == null) {
                details.append("Interface ").append(interfaceName).append(" not found.");
            } else {
                details.append("Interface Name: ").append(networkInterface.getName()).append("\n")
                        .append("Is Loopback: ").append(networkInterface.isLoopback()).append("\n")
                        .append("Supports Multicast: ").append(networkInterface.supportsMulticast()).append("\n")
                        .append("MTU: ").append(networkInterface.getMTU()).append("\n")
                        .append("Is Up: ").append(networkInterface.isUp()).append("\n");


                byte[] mac = networkInterface.getHardwareAddress();
                if (mac != null) {
                    details.append("MAC Address: ");
                    for (int i = 0; i < mac.length; i++) {
                        details.append(String.format("%02X%s", mac[i], (i < mac.length - 1) ? ":" : ""));
                    }
                    details.append("\n");
                } else {
                    details.append("MAC Address: Not available\n");
                }

                details.append("IPv6 Addresses:\n");
                Enumeration<InetAddress> addresses = networkInterface.getInetAddresses();
                boolean hasIPv6 = false;
                while (addresses.hasMoreElements()) {
                    InetAddress addr = addresses.nextElement();
                    if (addr instanceof Inet6Address) {
                        details.append("  - ").append(addr.getHostAddress()).append("\n");
                        hasIPv6 = true;
                    }
                }
                if (!hasIPv6) {
                    details.append("  No IPv6 addresses assigned.\n");
                }
            }
        } catch (SocketException e) {
            details.append("Error retrieving interface info: ").append(e.getMessage());
            Log.e("StreamingFragment", "SocketException", e);
        }

		// Show info in AlertDialog
        new AlertDialog.Builder(getContext())
                .setMessage(details.toString())
                .setPositiveButton("OK", null)
                .show();
    }


}
