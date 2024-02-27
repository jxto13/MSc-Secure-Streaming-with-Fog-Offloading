package com.example.final_test_wifiaware.ui.home;

import android.annotation.SuppressLint;
import android.content.Context;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;
import android.widget.Switch;

import androidx.annotation.NonNull;
import androidx.recyclerview.widget.RecyclerView;
import com.example.final_test_wifiaware.R;
import com.example.final_test_wifiaware.network.utils.Connection;
import com.example.final_test_wifiaware.utils.AppContainer;

import java.util.List;

public class CustomAdapter extends RecyclerView.Adapter<CustomAdapter.ViewHolder> {
	private final Context context;
	private List<Connection> connections;

	public CustomAdapter(Context context, List<Connection> connections) {
		this.context = context;
		this.connections = connections;
	}

	public void updateData(List<Connection> newConnections) {
		this.connections = newConnections;
	}


	@NonNull
	@Override
	public ViewHolder onCreateViewHolder(@NonNull ViewGroup parent, int viewType) {
		View itemView = LayoutInflater.from(context).inflate(R.layout.row_item, parent, false);
		return new ViewHolder(itemView);
	}

	@Override
	public void onBindViewHolder(ViewHolder holder, int position) {
		Connection connection = connections.get(position);

		holder.column1.setText(connection.getUUID().toString());
		holder.column2.setText(connection.getConnectedStatus().toString());

		holder.column3.setChecked(false);

		holder.column3.setOnCheckedChangeListener((buttonView, isChecked) -> {
			if (isChecked) {
				System.out.println("Switch " + connection.getUUID().toString() + " is on");
				//AppContainer.getInstance().getSubscriber().initializeSocketConnectionSubscriber(connection);
				// Handle when the Switch is checked (toggled on)
				// You can add your logic here
			} else {
				System.out.println("Switch " + connection.getUUID().toString() + " is off");
				//AppContainer.getInstance().getSubscriber().destroySpecificResource(connection);

				// Handle when the Switch is unchecked (toggled off)
				// You can add your logic here
			}
		});
	}

	@Override
	public int getItemCount() {
		return connections.size();
	}

	public static class ViewHolder extends RecyclerView.ViewHolder {
		public TextView column1;
		public TextView column2;
		@SuppressLint("UseSwitchCompatOrMaterialCode")
		public Switch column3;

		public ViewHolder(View itemView) {
			super(itemView);
			column1 = itemView.findViewById(R.id.column1);
			column2 = itemView.findViewById(R.id.column2);
			column3 = itemView.findViewById(R.id.column3);
		}
	}
}