package pt.iris.ui.slideshow;

import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import androidx.annotation.NonNull;
import androidx.annotation.OptIn;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;
import androidx.media3.common.util.UnstableApi;

import com.arthenica.ffmpegkit.FFmpegKitConfig;
import pt.iris.databinding.FragmentSlideshowBinding;
import pt.iris.services.RustService;

import org.videolan.libvlc.LibVLC;
import org.videolan.libvlc.Media;
import org.videolan.libvlc.MediaPlayer;
import org.videolan.libvlc.util.VLCVideoLayout;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;

public class SlideshowFragmentBackup extends Fragment {
	private static final boolean USE_TEXTURE_VIEW = false;
	private static final boolean ENABLE_SUBTITLES = true;

	private FragmentSlideshowBinding binding;
	private LibVLC libvlc;
	private MediaPlayer mMediaPlayer = null;
	private VLCVideoLayout videoLayout;
	private volatile boolean isWriting = true;
	private Thread fileWriterThread;

	@OptIn(markerClass = UnstableApi.class)
	public View onCreateView(@NonNull LayoutInflater inflater,
							 ViewGroup container, Bundle savedInstanceState) {
		SlideshowViewModel slideshowViewModel =
				new ViewModelProvider(this).get(SlideshowViewModel.class);

		binding = FragmentSlideshowBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		libvlc = new LibVLC(requireContext());
		mMediaPlayer = new MediaPlayer(libvlc);
		//videoLayout = binding.videoLayout;

		//mMediaPlayer.attachViews(videoLayout, null, ENABLE_SUBTITLES, USE_TEXTURE_VIEW);

		final String pipe = FFmpegKitConfig.registerNewFFmpegPipe(requireContext());
		Media media = new Media(libvlc, pipe);

		mMediaPlayer.setMedia(media);

		media.release();

		mMediaPlayer.play();

		// Start the file writing thread with the path as parameter
		startFileWritingThread(pipe);

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

		mMediaPlayer.stop();
		libvlc.release();
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
