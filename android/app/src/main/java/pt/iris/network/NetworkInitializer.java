package pt.iris.network;

import android.annotation.SuppressLint;
import android.content.Context;

import java.util.UUID;



@SuppressLint("MissingPermission")
public class NetworkInitializer {
	private static final String TAG = "NetworkInitializer";
	public static final String SERVICE_NAME = "Iris_Streaming_Service";

	private final Context context;
	private UUID myUUID;

	public NetworkInitializer(Context context) {
		this.context = context;
	}


}
