package pt.iris.ui.streaming;

import android.content.Context;
import android.net.wifi.aware.PeerHandle;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;

import androidx.annotation.NonNull;
import androidx.recyclerview.widget.RecyclerView;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.lang.reflect.Field;

import pt.iris.R;
import pt.iris.network.lib.WifiAwareModels.DataPathInfo;
import pt.iris.network.lib.WifiAwareModels.SessionInfo;

/**
 * RecyclerView adapter for displaying Wi-Fi Aware connections and discovered peers
 */
public class WifiAwareConnectionsAdapter extends RecyclerView.Adapter<WifiAwareConnectionsAdapter.ConnectionViewHolder> {
    private List<DataPathInfo> connections = new ArrayList<>();
    private OnConnectionClickListener listener;
    private SessionInfo sessionInfo;
    private boolean connectionsEnabled = false;
    // ...existing code...
    
    public interface OnConnectionClickListener {
        void onConnectionClick(DataPathInfo connection);
        void onConnectPeer(DataPathInfo peer);
    }
    
    public void setOnConnectionClickListener(OnConnectionClickListener listener) {
        this.listener = listener;
    }
    
    public void updateConnections(Map<PeerHandle, DataPathInfo> connectionsMap) {
        Log.d("mine", "[WifiAwareConnectionsAdapter] updateConnections called. Map size: " + (connectionsMap != null ? connectionsMap.size() : 0));
        connections.clear();
        if (connectionsMap != null) {
            for (DataPathInfo info : connectionsMap.values()) {
                Log.d("mine", "[WifiAwareConnectionsAdapter] Adding connection: peerHandle=" + info.peerHandle + ", ipAddress=" + info.ipAddress + ", isActive=" + info.isActive + ", isConnected=" + info.isConnected);
                connections.add(info);
            }
        }
        Log.d("mine", "[WifiAwareConnectionsAdapter] Adapter connections list size: " + connections.size());
        notifyDataSetChanged(); // Force UI refresh
    }

    public void updateSessionInfo(SessionInfo sessionInfo) {
        this.sessionInfo = sessionInfo;
        notifyDataSetChanged();
    }
    
    public void setConnectionsEnabled(boolean enabled) {
        this.connectionsEnabled = enabled;
        notifyDataSetChanged();
    }
    
    public void addConnection(DataPathInfo connection) {
        // Check if connection already exists (by IP address)
        for (int i = 0; i < connections.size(); i++) {
            if (connections.get(i).ipAddress.equals(connection.ipAddress)) {
                connections.set(i, connection);
                notifyItemChanged(i);
                return;
            }
        }
        
        // Add new connection
        connections.add(connection);
        notifyItemInserted(connections.size() - 1);
    }
    
    public void removeConnection(PeerHandle peerHandle) {
        for (int i = 0; i < connections.size(); i++) {
            if (connections.get(i).peerHandle.equals(peerHandle)) {
                connections.remove(i);
                notifyItemRemoved(i);
                break;
            }
        }
    }
    
    public void clearConnections() {
        int size = connections.size();
        connections.clear();
        notifyItemRangeRemoved(0, size);
    }
    
    @NonNull
    @Override
    public ConnectionViewHolder onCreateViewHolder(@NonNull ViewGroup parent, int viewType) {
        View view = LayoutInflater.from(parent.getContext())
                .inflate(R.layout.item_wifi_aware_connection, parent, false);
        return new ConnectionViewHolder(view);
    }

    @Override
    public void onBindViewHolder(@NonNull ConnectionViewHolder holder, int position) {
        DataPathInfo connection = connections.get(position);
        Log.d("WifiAwareAdapter", "onBindViewHolder: position=" + position + ", peerHandle=" + connection.peerHandle + ", ipAddress=" + connection.ipAddress + ", isActive=" + connection.isActive);
        holder.bind(connection, sessionInfo, null);
    }

    
    @Override
    public int getItemCount() {
        return connections.size();
    }
    
    // Helper to extract mPeerId from PeerHandle via reflection
    private Integer getPeerId(PeerHandle peerHandle) {
        if (peerHandle == null) return null;
        try {
            Field field = PeerHandle.class.getDeclaredField("mPeerId");
            field.setAccessible(true);
            return field.getInt(peerHandle);
        } catch (Exception e) {
            Log.w("WifiAwareAdapter", "Failed to get peerId from PeerHandle: " + e);
            return null;
        }
    }

    class ConnectionViewHolder extends RecyclerView.ViewHolder {
        private final TextView peerHandleText;
        private final TextView statusText;
        private final TextView ipAddressText;
        private final TextView durationText;
        
        public ConnectionViewHolder(@NonNull View itemView) {
            super(itemView);
            peerHandleText = itemView.findViewById(R.id.peerHandleText);
            statusText = itemView.findViewById(R.id.statusText);
            ipAddressText = itemView.findViewById(R.id.ipAddressText);
            durationText = itemView.findViewById(R.id.durationText);
            
            itemView.setOnClickListener(v -> {
                if (listener != null && getAdapterPosition() != RecyclerView.NO_POSITION) {
                    DataPathInfo connection = connections.get(getAdapterPosition());
                    Log.d("WifiAwareAdapter", "Item clicked - Position: " + getAdapterPosition() + 
                          ", Connected: " + connection.isConnected() + 
                          ", Peer: " + connection.peerHandle);
                    
                    if (connection.isConnected()) {
                        Log.d("WifiAwareAdapter", "Calling onConnectionClick for connected peer");
                        listener.onConnectionClick(connection);
                    } else {
                        // Always call onConnectPeer for non-connected peers
                        // Let the fragment handle the logic for whether to show dialog or toast
                        Log.d("WifiAwareAdapter", "Calling onConnectPeer for non-connected peer");
                        listener.onConnectPeer(connection);
                    }
                }
            });
            

        }

        
        public void bind(DataPathInfo connection, SessionInfo sessionInfo, pt.iris.network.lib.WifiAwareModels.DeviceRole unusedRole) {
            // Display peer UUID if available, otherwise fallback to peerHandle
            String displayStr;
            if (connection.peerUuid != null) {
                String uuidStr = connection.peerUuid.toString();
                displayStr = "UUID: " + uuidStr.substring(0, 8) + "...";
            } else {
                String peerHandleStr = connection.peerHandle.toString();
                if (peerHandleStr.length() > 10) {
                    peerHandleStr = "Peer: " + peerHandleStr.substring(peerHandleStr.length() - 6);
                } else {
                    peerHandleStr = "Peer: " + peerHandleStr;
                }
                displayStr = peerHandleStr;
            }
            peerHandleText.setText(displayStr);

            // Connection Status
            String connectionStatus = connection.getConnectionStatus();
            statusText.setText(connectionStatus);
            statusText.setTextColor(itemView.getContext().getColor(
                    connection.isConnected() ? android.R.color.holo_green_light : android.R.color.holo_orange_light));

            // IP Address and role info
            String addressInfo = connection.getDisplayAddress();
            ipAddressText.setText(addressInfo);
            // ...existing code...

            // Only display duration after connection
            if (connection.isConnected()) {
                durationText.setText("Duration: " + connection.getDurationString());
            } else {
                durationText.setText("");
            }

            // Update card background color based on status
            if (connection.isConnected()) {
                itemView.setBackgroundColor(itemView.getContext().getColor(android.R.color.holo_green_light));
            } else if (!connectionsEnabled) {
                itemView.setBackgroundColor(itemView.getContext().getColor(android.R.color.darker_gray));
            } else {
                itemView.setBackgroundColor(itemView.getContext().getColor(android.R.color.holo_orange_light));
            }
        }
    }
}