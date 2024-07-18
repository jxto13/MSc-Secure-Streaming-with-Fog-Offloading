package pt.iris.ui.slideshow;

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

import com.arthenica.ffmpegkit.FFmpegKitConfig;

import pt.iris.databinding.FragmentSlideshowBinding;
import pt.iris.services.RustService;

import java.util.ArrayList;

public class SlideshowFragment extends Fragment {

	private static final boolean USE_TEXTURE_VIEW = false;
	private static final boolean ENABLE_SUBTITLES = false;
	private VLCVideoLayout mVideoLayout = null;
	private LibVLC mLibVLC = null;
	private MediaPlayer mMediaPlayer = null;

	private FragmentSlideshowBinding binding;

	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		SlideshowViewModel slideshowViewModel = new ViewModelProvider(this).get(SlideshowViewModel.class);

		binding = FragmentSlideshowBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		// Create pipe to send to JNI
		String pipe = FFmpegKitConfig.registerNewFFmpegPipe(getContext());

		// Create a new Thread to run the JNI to copy decoded bytes to Video Player
		Thread thread = new Thread(() -> RustService.checkWritePermission(pipe));
		thread.start();

		// Args for VLC Player
		final ArrayList<String> args = new ArrayList<>();
		args.add("-vvv");

		// Create VLC Player
		mLibVLC = new LibVLC(getContext(), args);
		mMediaPlayer = new MediaPlayer(mLibVLC);

		mVideoLayout = binding.videoLayout;
		mMediaPlayer.attachViews(mVideoLayout, null, ENABLE_SUBTITLES, USE_TEXTURE_VIEW);

		// Create Media
		//final Media media = new Media(mLibVLC, pipe);
		final Media media = new Media(mLibVLC, "udp://127.0.0.1:12346");

		System.out.println(media.getUri());
		mMediaPlayer.setMedia(media);

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
