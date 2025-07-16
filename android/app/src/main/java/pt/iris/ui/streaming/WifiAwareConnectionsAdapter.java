package pt.iris.ui.streaming;

import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;
import androidx.annotation.NonNull;
import androidx.recyclerview.widget.RecyclerView;
import pt.iris.R;
import pt.iris.network.lib.WifiAwareModels;

import java.util.ArrayList;
import java.util.List;

public class WifiAwareConnectionsAdapter extends RecyclerView.Adapter<WifiAwareConnectionsAdapter.ConnectionViewHolder> {

    private List<WifiAwareModels.ConnectionInfo> connections = new ArrayList<>();
    private OnConnectionClickListener onConnectionClickListener;

    public interface OnConnectionClickListener {
        void onConnectionClick(WifiAwareModels.ConnectionInfo connection);
    }

    public void setOnConnectionClickListener(OnConnectionClickListener listener) {
        this.onConnectionClickListener = listener;
    }

    public void updateConnections(List<WifiAwareModels.ConnectionInfo> newConnections) {
        this.connections.clear();
        this.connections.addAll(newConnections);
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
        WifiAwareModels.ConnectionInfo connection = connections.get(position);
        holder.bind(connection);
        
        holder.itemView.setOnClickListener(v -> {
            if (onConnectionClickListener != null) {
                onConnectionClickListener.onConnectionClick(connection);
            }
        });
    }

    @Override
    public int getItemCount() {
        return connections.size();
    }

    static class ConnectionViewHolder extends RecyclerView.ViewHolder {
        private TextView peerHandleText;
        private TextView ipAddressText;
        private TextView durationText;
        private TextView statusText;

        public ConnectionViewHolder(@NonNull View itemView) {
            super(itemView);
            peerHandleText = itemView.findViewById(R.id.peerHandleText);
            ipAddressText = itemView.findViewById(R.id.ipAddressText);
            durationText = itemView.findViewById(R.id.durationText);
            statusText = itemView.findViewById(R.id.statusText);
        }

        public void bind(WifiAwareModels.ConnectionInfo connection) {
            String[] parts = connection.peerHandleStr.split("\\.");
            String lastWord = parts[parts.length - 1];
            peerHandleText.setText("Peer: " + lastWord);
            ipAddressText.setText(connection.ipAddress);
            
            long ageSeconds = (System.currentTimeMillis() - connection.createdTime) / 1000;
            durationText.setText("Duration: " + ageSeconds + "s");
            
            String status = connection.isActive ? "Active" : "Inactive";
            statusText.setText(status);
            
            // Color code status
            int statusColor = connection.isActive ? 
                android.R.color.holo_green_dark : android.R.color.holo_red_dark;
            statusText.setTextColor(itemView.getContext().getColor(statusColor));
        }
    }
}
