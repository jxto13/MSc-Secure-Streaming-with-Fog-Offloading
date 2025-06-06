package pt.iris.ui.slideshow;

import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import androidx.annotation.NonNull;
import androidx.annotation.OptIn;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;
import androidx.media3.common.C;
import androidx.media3.common.MediaItem;
import androidx.media3.common.util.UnstableApi;
import androidx.media3.datasource.DataSource;
import androidx.media3.datasource.DataSpec;
import androidx.media3.datasource.TransferListener;
import androidx.media3.exoplayer.ExoPlayer;
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory;

import java.io.FileInputStream;
import java.io.IOException;
import pt.iris.databinding.FragmentSlideshowBinding;
import pt.iris.services.RustService;

import android.net.Uri;
import androidx.annotation.Nullable;

import java.nio.ByteBuffer;

@UnstableApi
class PfdPipeDataSource implements DataSource {
	private final ParcelFileDescriptor readPfd;
	private final FileInputStream inputStream;
	@Nullable private TransferListener transferListener;
	@Nullable private DataSpec dataSpec;
	private boolean opened = false;

	public PfdPipeDataSource(ParcelFileDescriptor readPfd) {
		this.readPfd = readPfd;
		this.inputStream = new FileInputStream(readPfd.getFileDescriptor());
	}

	public static class Factory implements DataSource.Factory {
		private final ParcelFileDescriptor readPfd;

		public Factory(ParcelFileDescriptor readPfd) {
			this.readPfd = readPfd;
		}

		@NonNull
		@Override
		public DataSource createDataSource() {
			return new PfdPipeDataSource(readPfd);
		}
	}

	@Override
	public void addTransferListener(@NonNull TransferListener transferListener) {
		this.transferListener = transferListener;
	}

	@Override
	public long open(@NonNull DataSpec dataSpec) throws IOException {
		this.dataSpec = dataSpec;
		opened = true;
		if (transferListener != null) {
			transferListener.onTransferStart(this, dataSpec, true);
		}
		return C.LENGTH_UNSET;
	}

	@Override
	public int read(@NonNull byte[] target, int offset, int length) throws IOException {
		if (!opened) return -1;
		int bytesRead = inputStream.read(target, offset, length);
		if (bytesRead > 0 && transferListener != null) {
			assert dataSpec != null;
			transferListener.onBytesTransferred(this, dataSpec, true, bytesRead);
		}
		return bytesRead;
	}

	@Override
	public @Nullable Uri getUri() {
		return dataSpec != null ? dataSpec.uri : null;
	}

	@Override
	public void close() {
		opened = false;
		if (transferListener != null && dataSpec != null) {
			transferListener.onTransferEnd(this, dataSpec, true);
		}
		try {
			inputStream.close();
			readPfd.close();
		} catch (IOException ignored) {}
	}
}

public class SlideshowFragment extends Fragment {

	private FragmentSlideshowBinding binding;

	@OptIn(markerClass = UnstableApi.class)
	@Override
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		SlideshowViewModel slideshowViewModel = new ViewModelProvider(this).get(SlideshowViewModel.class);

		binding = FragmentSlideshowBinding.inflate(inflater, container, false);
		View root = binding.getRoot();
		
		ParcelFileDescriptor[] pipe = null;
		try {
			pipe = ParcelFileDescriptor.createPipe();
		} catch (IOException e) {
			throw new RuntimeException(e);
		}
		ParcelFileDescriptor readPfd = pipe[0];
		ParcelFileDescriptor writePfd = pipe[1];

		// Pass writePfd.getFd() to native code for writing
		int fdForNative = writePfd.getFd();

		RustService.startClient(fdForNative);

		ExoPlayer player = new ExoPlayer.Builder(root.getContext()).setMediaSourceFactory(
				new DefaultMediaSourceFactory(
						new PfdPipeDataSource.Factory(readPfd)
				)
		).build();

		binding.playerView.setPlayer(player);
		// Replace with your custom URI
		Uri uri = Uri.parse("iris://192.168.1.131:1234");
		MediaItem mediaItem = MediaItem.fromUri(uri);
		player.setMediaItem(mediaItem);
		player.prepare();
		player.play();

		return root;
	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		binding = null;
	}


}
