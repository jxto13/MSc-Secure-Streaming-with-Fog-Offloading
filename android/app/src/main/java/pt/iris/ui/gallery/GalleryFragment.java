package pt.iris.ui.gallery;

import android.content.Context;
import android.net.Uri;
import android.os.Bundle;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import androidx.annotation.NonNull;
import androidx.annotation.OptIn;
import androidx.fragment.app.Fragment;
import androidx.lifecycle.ViewModelProvider;

import org.videolan.libvlc.LibVLC;
import org.videolan.libvlc.Media;
import org.videolan.libvlc.MediaPlayer;
import org.videolan.libvlc.util.VLCVideoLayout;


import java.io.IOException;
import java.util.ArrayList;

import pt.iris.databinding.FragmentGalleryBinding;

public class GalleryFragment extends Fragment {

	private static final boolean USE_TEXTURE_VIEW = false;
	private static final boolean ENABLE_SUBTITLES = true;
	private static final String ASSET_FILENAME = "bbb.m4v";

	private VLCVideoLayout mVideoLayout = null;

	private LibVLC mLibVLC = null;
	private MediaPlayer mMediaPlayer = null;

	private FragmentGalleryBinding binding;

	@Override
	public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
		GalleryViewModel slideshowViewModel = new ViewModelProvider(this).get(GalleryViewModel.class);

		binding = FragmentGalleryBinding.inflate(inflater, container, false);
		View root = binding.getRoot();

		Context context = root.getContext();

		final ArrayList<String> args = new ArrayList<>();
		args.add("-vvv");

		mLibVLC = new LibVLC(context, args);

		mMediaPlayer = new MediaPlayer(mLibVLC);

		mVideoLayout = binding.videoLayout;

		return root;
	}

	@Override
	public void onStart() {
		super.onStart();

		mMediaPlayer.attachViews(mVideoLayout, null, ENABLE_SUBTITLES, USE_TEXTURE_VIEW);
		Uri uri = Uri.parse("http://127.0.0.1:9000");

		final Media media = new Media(mLibVLC, uri);
		mMediaPlayer.setMedia(media);
		media.release();

		mMediaPlayer.play();

	}

	@Override
	public void onDestroyView() {
		super.onDestroyView();
		mMediaPlayer.release();
		mLibVLC.release();

	}

}
