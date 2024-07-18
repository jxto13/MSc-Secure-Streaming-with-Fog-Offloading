package pt.iris.ui.gallery;

import static android.content.ContentValues.TAG;

import android.net.Uri;
import android.os.Bundle;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import androidx.annotation.NonNull;
import androidx.annotation.OptIn;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;


import com.arthenica.ffmpegkit.FFmpegKitConfig;
import com.google.android.exoplayer2.ExoPlayer;
import com.google.android.exoplayer2.MediaItem;
import com.google.android.exoplayer2.ui.PlayerView;

import pt.iris.databinding.FragmentGalleryBinding;
import pt.iris.services.RustService;

public class GalleryFragment extends Fragment {

	private FragmentGalleryBinding binding;
	private ExoPlayer player;

    @Override
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		GalleryViewModel slideshowViewModel = new ViewModelProvider(this).get(GalleryViewModel.class);

		binding = FragmentGalleryBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		final String pipe1 = FFmpegKitConfig.registerNewFFmpegPipe(requireContext());
		// Create pipe to send to JNI
		String pipe = FFmpegKitConfig.registerNewFFmpegPipe(getContext());

		// Create a new Thread to run the JNI to copy decoded bytes to Video Player
		Thread thread = new Thread(() -> RustService.checkWritePermission(pipe));
		thread.start();
		// Initialize ExoPlayer
		player = new ExoPlayer.Builder(requireContext())
				.build();

		PlayerView playerView = binding.playerView;
		playerView.setPlayer(player);
		//playerView.setUseController(false);//set to true or false to see controllers

		player.setMediaItem(MediaItem.fromUri("udp://127.0.0.1:12346"));
		player.prepare();
		player.play();



		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		player.stop();
		player.release();
		binding = null;
	}
}
