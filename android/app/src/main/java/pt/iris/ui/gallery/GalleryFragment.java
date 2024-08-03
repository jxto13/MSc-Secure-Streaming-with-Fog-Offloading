package pt.iris.ui.gallery;

import android.net.Uri;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;
import androidx.media3.common.MediaItem;
import androidx.media3.common.MimeTypes;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.ui.PlayerView;

//import com.arthenica.ffmpegkit.FFmpegKitConfig;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;

import pt.iris.databinding.FragmentGalleryBinding;
import pt.iris.services.RustService;

public class GalleryFragment extends Fragment {

	private FragmentGalleryBinding binding;

	@Override
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		GalleryViewModel slideshowViewModel = new ViewModelProvider(this).get(GalleryViewModel.class);

		binding = FragmentGalleryBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Create pipe to send to JNI
		//String pipe = FFmpegKitConfig.registerNewFFmpegPipe(getContext());

		//System.out.println(pipe);

		ExoPlayer player = new ExoPlayer.Builder(this.getContext()).build();

		PlayerView playerView = binding.playerView;
		playerView.setPlayer(player);

		// Build the media item.
		MediaItem mediaItem = MediaItem.fromUri("udp://127.0.0.1:12346");

		// Set the media item to be played.
		player.setMediaItem(mediaItem);

		// Prepare the player.
		player.prepare();

		// Start the playback.
		player.play();

		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
	}
}
