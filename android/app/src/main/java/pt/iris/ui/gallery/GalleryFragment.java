package pt.iris.ui.gallery;

import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import androidx.annotation.NonNull;
import androidx.annotation.OptIn;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import androidx.media3.common.MediaItem;
import androidx.media3.common.util.UnstableApi;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.ui.PlayerView;


import pt.iris.databinding.FragmentGalleryBinding;

public class GalleryFragment extends Fragment {

	private FragmentGalleryBinding binding;
	private ExoPlayer player;

	@OptIn(markerClass = UnstableApi.class)
	@Override
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		GalleryViewModel slideshowViewModel = new ViewModelProvider(this).get(GalleryViewModel.class);

		binding = FragmentGalleryBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		PlayerView playerView = binding.media3Player;

		// Initialize ExoPlayer
		player = new ExoPlayer.Builder(getContext()).build();
		playerView.setPlayer(player);

		// Prepare and play
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
		if (player != null) {
			player.release();
			player = null;
		}
	}

}
