package com.example.final_test_wifiaware.ui.slideshow;

import static androidx.fragment.app.FragmentManager.TAG;

import android.annotation.SuppressLint;
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
import androidx.media3.common.C;
import androidx.media3.common.MediaItem;
import androidx.media3.common.PlaybackException;
import androidx.media3.common.Player;
import androidx.media3.common.util.UnstableApi;
import androidx.media3.datasource.DataSource;
import androidx.media3.datasource.DefaultDataSource;
import androidx.media3.exoplayer.DefaultLoadControl;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.exoplayer.source.MediaSource;
import androidx.media3.exoplayer.source.ProgressiveMediaSource;
import androidx.media3.ui.PlayerView;

import com.arthenica.ffmpegkit.FFmpegKitConfig;
import com.example.final_test_wifiaware.databinding.FragmentSlideshowBinding;
import com.example.final_test_wifiaware.services.RustService;

import org.videolan.libvlc.Media;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;

public class SlideshowFragment extends Fragment {

	private FragmentSlideshowBinding binding;
	private ExoPlayer player;

	private Thread fileWriterThread;
	private volatile boolean isWriting = true;

	@UnstableApi
	class CustomDataSourceFactory implements DataSource.Factory {

		@Override
		public DataSource createDataSource() {
			//return new CustomDataSource();
			return new ByteArrayDataSourceCustom();
			//return new UdpDataSource();
		}
	}

	@OptIn(markerClass = UnstableApi.class)
	public View onCreateView(@NonNull LayoutInflater inflater,
							 ViewGroup container, Bundle savedInstanceState) {
		SlideshowViewModel slideshowViewModel =
				new ViewModelProvider(this).get(SlideshowViewModel.class);

		binding = FragmentSlideshowBinding.inflate(inflater, container, false);
		View root = binding.getRoot();


		player = new ExoPlayer.Builder(getContext())
				.setLoadControl(new DefaultLoadControl.Builder()
						.setTargetBufferBytes(C.LENGTH_UNSET) // Target buffer size for live streams
						.build())
				.build();

		PlayerView playerView = binding.videoLayout;
		playerView.setPlayer(player);

		player.addListener(new Player.Listener() {
			@Override
			public void onPlaybackStateChanged(int playbackState) {
				Player.Listener.super.onPlaybackStateChanged(playbackState);
				System.out.println("playbackState " + playbackState);

			}

			@Override
			public void onPlayerError(PlaybackException error) {
				Player.Listener.super.onPlayerError(error);
				System.out.println("error " + error);
			}
		});

		// Add a listener for player events
		player.addListener(new Player.Listener() {
			@SuppressLint("RestrictedApi")
			@Override
			public void onPlaybackStateChanged(int playbackState) {
				switch (playbackState) {
					case ExoPlayer.STATE_IDLE:
						Log.d(TAG, "Player state: STATE_IDLE");
						break;
					case ExoPlayer.STATE_BUFFERING:
						Log.d(TAG, "Player state: STATE_BUFFERING");
						break;
					case ExoPlayer.STATE_READY:
						Log.d(TAG, "Player state: STATE_READY");
						break;
					case ExoPlayer.STATE_ENDED:
						Log.d(TAG, "Player state: STATE_ENDED");
						break;
					default:
						Log.d(TAG, "Player state: UNKNOWN");
						break;
				}
			}

			@SuppressLint("RestrictedApi")
			@Override
			public void onPlayerError(PlaybackException error) {
				Log.e(TAG, "Player error: " + error.getMessage());
			}
		});


		// Create a MediaSource using the custom DataSource
		Uri uri = Uri.parse("udp://192.168.1.131:1234"); // Custom URI for your data source

		DataSource.Factory dataSourceFactory = new CustomDataSourceFactory();


		MediaItem mediaItem =
				new MediaItem.Builder()
						.setUri("mediaUri")
						.setLiveConfiguration(
								new MediaItem.LiveConfiguration.Builder().setMaxPlaybackSpeed(1.02f).build())
						.build();

		MediaSource mediaSource = new ProgressiveMediaSource.Factory(dataSourceFactory).createMediaSource(mediaItem);

		player.setMediaSource(mediaSource);

		final String pipe = FFmpegKitConfig.registerNewFFmpegPipe(requireContext());


		// Build the media item.
		//MediaItem mediaItem2 = MediaItem.fromUri("/data/data/com.example.final_test_wifiaware/files/SampleVideo_1280x720_20mb.mp4");
		//MediaItem mediaItem = MediaItem.fromUri("/data/data/com.example.final_test_wifiaware/files/fileSequence0.ts");
		//MediaItem mediaItem2 = MediaItem.fromUri(pipe);


		// Set the media item to be played.
		//player.setMediaItem(mediaItem2);

		// Prepare the player.
		player.prepare();
		// Start the playback.
		player.play();

		// Start the file writing thread with the path as parameter
		//startFileWritingThread(pipe);
		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		// Stop the file writing thread
		isWriting = false;
		if (fileWriterThread != null) {
			try {
				fileWriterThread.join();
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
		}

		player.stop();
		player.release();
		binding = null;
	}

	// Method to start the file writing thread
	private void startFileWritingThread(String filePath) {
		Runnable fileWriterRunnable = new Runnable() {
			@Override
			public void run() {
				writeFile(filePath);
			}
		};

		fileWriterThread = new Thread(fileWriterRunnable);
		fileWriterThread.start();
	}

	// Method to write to a file
	private void writeFile(String filePath) {
		File file = new File(filePath);
		FileOutputStream fos = null;
		try {
			fos = new FileOutputStream(file, true); // 'true' to append to the file
			while (isWriting) {
				fos.write(RustService.helloByte(188 * 5));
				fos.flush();
				//Thread.sleep(1000);
			}
			System.out.println("Write operation completed successfully.");
		} catch (IOException e) {
			// Handle the exception, but avoid printing stack trace in production code
			if (isWriting) {
				e.printStackTrace();
			}
		} finally {
			if (fos != null) {
				try {
					fos.close();
				} catch (IOException e) {
					e.printStackTrace();
				}
			}
		}
	}
}
