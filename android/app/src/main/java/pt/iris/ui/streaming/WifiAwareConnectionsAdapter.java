package pt.iris.ui.streaming;

import android.annotation.SuppressLint;
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
 * RecyclerView adapter for displaying Wi-Fi Aware active connections only
 */
public class WifiAwareConnectionsAdapter extends RecyclerView.Adapter<WifiAwareConnectionsAdapter.ConnectionViewHolder> {
    private List<DataPathInfo> connections = new ArrayList<>();
    private OnConnectionClickListener listener;
    private SessionInfo sessionInfo;
    private boolean connectionsEnabled = false;
    private final String TAG = "WifiAwareConnectionsAdapter";

    public interface OnConnectionClickListener {
        void onConnectionClick(DataPathInfo connection);
        void onConnectPeer(DataPathInfo peer);
    }
    
    public void setOnConnectionClickListener(OnConnectionClickListener listener) {
        this.listener = listener;
    }
    
    public void updateConnections(Map<PeerHandle, DataPathInfo> connectionsMap) {
        Log.d(TAG, "updateConnections called. Map size: " + (connectionsMap != null ? connectionsMap.size() : 0));
        connections.clear();
        if (connectionsMap != null) {
            connections.addAll(connectionsMap.values());
        }
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
    


    class ConnectionViewHolder extends RecyclerView.ViewHolder {
        private final TextView peerHandleText;
        private final TextView ipAddressText;
        private final TextView durationText;
        
        public ConnectionViewHolder(@NonNull View itemView) {
            super(itemView);
            peerHandleText = itemView.findViewById(R.id.peerHandleText);
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

        
        @SuppressLint("SetTextI18n")
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