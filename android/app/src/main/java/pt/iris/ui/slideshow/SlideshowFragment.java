package pt.iris.ui.slideshow;
/*

import android.net.Uri;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import org.videolan.libvlc.LibVLC;
import org.videolan.libvlc.Media;
import org.videolan.libvlc.MediaPlayer;
import org.videolan.libvlc.util.VLCVideoLayout;

//import com.arthenica.ffmpegkit.FFmpegKitConfig;

import pt.iris.databinding.FragmentSlideshowBinding;
import pt.iris.services.RustService;

import java.io.FileDescriptor;
import java.io.FileInputStream;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public class SlideshowFragment extends Fragment {

	private static final boolean USE_TEXTURE_VIEW = false;
	private static final boolean ENABLE_SUBTITLES = true;
	private VLCVideoLayout mVideoLayout = null;
	private LibVLC mLibVLC = null;
	private MediaPlayer mMediaPlayer = null;

	private FragmentSlideshowBinding binding;

	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		SlideshowViewModel slideshowViewModel = new ViewModelProvider(this).get(SlideshowViewModel.class);

		binding = FragmentSlideshowBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Create pipe to send to JNI
		//String pipe = FFmpegKitConfig.registerNewFFmpegPipe(getContext());

		// Create a new Thread to run the JNI to copy decoded bytes to Video Player
		//Thread thread = new Thread(() -> RustService.checkWritePermission(pipe));
		//thread.start();

		RustService.checkWritePermission("pipe");

		// Args for VLC Player
		final ArrayList<String> args = new ArrayList<>();
		args.add("-vvv");
		*/
/*args.add("-file-caching=150");
		args.add("-network-caching=150");
		args.add("-clock-jitter=0");
		args.add("-live-caching=150");
		args.add("-drop-late-frames");
		args.add("-skip-frames");
		args.add("-vout=android-display");
		args.add("-sout-transcode-vb=20");
		args.add("-no-audio");
		args.add("-sout=#transcode{vcodec=h264,vb=20,acodec=mpga,ab=128,channels=2,samplerate=44100}:duplicate{dst=display}");
		args.add("-sout-x264-nf");*//*


		// Create VLC Player
		mLibVLC = new LibVLC(getContext(), args);
		mMediaPlayer = new MediaPlayer(mLibVLC);

		mVideoLayout = binding.videoLayout;
		mMediaPlayer.attachViews(mVideoLayout, null, ENABLE_SUBTITLES, USE_TEXTURE_VIEW);

        // Create Media
		//final Media media = new Media(mLibVLC, pipe);
		//final Media media = new Media(mLibVLC, Uri.parse("udp://127.0.0.1:12346"));

		//System.out.println(media.getUri());
		//mMediaPlayer.setMedia(media);

		//media.release();

		mMediaPlayer.play();

		return root;
	}


	@Override
	public void onDestroyView() {
		super.onDestroyView();
		mMediaPlayer.release();
		mLibVLC.release();

		binding = null;
	}
}
*/


import android.net.Uri;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import org.videolan.libvlc.LibVLC;
import org.videolan.libvlc.Media;
import org.videolan.libvlc.MediaPlayer;
import org.videolan.libvlc.util.VLCVideoLayout;


import pt.iris.databinding.FragmentSlideshowBinding;
import pt.iris.services.RustService;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.ArrayList;

public class SlideshowFragment extends Fragment {

	private static final boolean USE_TEXTURE_VIEW = false;
	private static final boolean ENABLE_SUBTITLES = true;
	private static final String ASSET_FILENAME = "bird42.m2ts";

	private VLCVideoLayout mVideoLayout = null;

	private LibVLC mLibVLC = null;
	private MediaPlayer mMediaPlayer = null;


	private FragmentSlideshowBinding binding;

	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		SlideshowViewModel slideshowViewModel = new ViewModelProvider(this).get(SlideshowViewModel.class);

		binding = FragmentSlideshowBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Args for VLC Player
		final ArrayList<String> args = new ArrayList<>();
		args.add("-vvv");

		// Create VLC Player
		mLibVLC = new LibVLC(getContext(), args);
		mMediaPlayer = new MediaPlayer(mLibVLC);

		mVideoLayout = binding.videoLayout;

		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		mMediaPlayer.release();
		mLibVLC.release();
		binding = null;
	}

	@Override
	public void onStart() {
		super.onStart();

		mMediaPlayer.attachViews(mVideoLayout, null, ENABLE_SUBTITLES, USE_TEXTURE_VIEW);

        final Media media = new Media(mLibVLC, Uri.parse("pipe://data/data/pt.iris/cache/pipes/pipe1"));

        mMediaPlayer.setMedia(media);
        media.release();
        mMediaPlayer.play();
	}

	@Override
	public void onStop() {
		super.onStop();

		mMediaPlayer.stop();
		mMediaPlayer.detachViews();
	}

}
