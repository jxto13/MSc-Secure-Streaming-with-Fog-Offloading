package pt.iris.ui.streaming;

import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;
import androidx.annotation.NonNull;
import androidx.recyclerview.widget.RecyclerView;
import pt.iris.R;

import java.util.ArrayList;
import java.util.List;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

public class WifiAwareClientsAdapter extends RecyclerView.Adapter<WifiAwareClientsAdapter.ClientViewHolder> {

    private List<StreamingViewModel.WifiAwareClientInfo> clients = new ArrayList<>();
    private OnClientClickListener onClientClickListener;

    public interface OnClientClickListener {
        void onClientClick(StreamingViewModel.WifiAwareClientInfo client);
    }

    public void setOnClientClickListener(OnClientClickListener listener) {
        this.onClientClickListener = listener;
    }

    public void updateClients(List<StreamingViewModel.WifiAwareClientInfo> newClients) {
        this.clients.clear();
        this.clients.addAll(newClients);
        notifyDataSetChanged();
    }

    @NonNull
    @Override
    public ClientViewHolder onCreateViewHolder(@NonNull ViewGroup parent, int viewType) {
        View view = LayoutInflater.from(parent.getContext())
                .inflate(R.layout.item_wifi_aware_client, parent, false);
        return new ClientViewHolder(view);
    }

    @Override
    public void onBindViewHolder(@NonNull ClientViewHolder holder, int position) {
        StreamingViewModel.WifiAwareClientInfo client = clients.get(position);
        holder.bind(client);
        
        holder.itemView.setOnClickListener(v -> {
            if (onClientClickListener != null) {
                onClientClickListener.onClientClick(client);
            }
        });
    }

    @Override
    public int getItemCount() {
        return clients.size();
    }

    static class ClientViewHolder extends RecyclerView.ViewHolder {
        private TextView ipAddressText;
        private TextView macAddressText;
        private TextView bytesReceivedText;
        private TextView lastSeenText;
        private TextView statusText;

        public ClientViewHolder(@NonNull View itemView) {
            super(itemView);
            ipAddressText = itemView.findViewById(R.id.ipAddressText);
            macAddressText = itemView.findViewById(R.id.macAddressText);
            bytesReceivedText = itemView.findViewById(R.id.bytesReceivedText);
            lastSeenText = itemView.findViewById(R.id.lastSeenText);
            statusText = itemView.findViewById(R.id.statusText);
        }

        public void bind(StreamingViewModel.WifiAwareClientInfo client) {
            ipAddressText.setText(client.ipAddress);
            macAddressText.setText(client.macAddress);
            bytesReceivedText.setText(formatBytes(client.bytesReceived));
            
            SimpleDateFormat timeFormat = new SimpleDateFormat("HH:mm:ss", Locale.getDefault());
            lastSeenText.setText(timeFormat.format(new Date(client.lastSeen)));
            
            statusText.setText(client.connectionStatus);
            
            // Color code status
            int statusColor = client.connectionStatus.equals("Connected") ? 
                android.R.color.holo_green_dark : android.R.color.holo_red_dark;
            statusText.setTextColor(itemView.getContext().getColor(statusColor));
        }

        private String formatBytes(long bytes) {
            if (bytes < 1024) return bytes + " B";
            int exp = (int) (Math.log(bytes) / Math.log(1024));
            String pre = "KMGTPE".charAt(exp - 1) + "";
            return String.format("%.1f %sB", bytes / Math.pow(1024, exp), pre);
        }
    }
}
