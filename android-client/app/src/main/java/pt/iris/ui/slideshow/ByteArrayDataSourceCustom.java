package pt.iris.ui.slideshow;

import static java.lang.Math.min;

import android.net.Uri;

import androidx.annotation.Nullable;
import androidx.media3.common.C;
import androidx.media3.common.util.UnstableApi;
import androidx.media3.datasource.BaseDataSource;
import androidx.media3.datasource.DataSpec;

import pt.iris.services.RustService;

import java.io.IOException;

@UnstableApi
public final class ByteArrayDataSourceCustom extends BaseDataSource {

	private byte[] data;
	@Nullable
	private Uri uri;
	private boolean opened;
	private int packetRemaining;

	private int total;
	/** The default maximum datagram packet size, in bytes. */
	public static final int DEFAULT_MAX_PACKET_SIZE = 2000;

	public ByteArrayDataSourceCustom() {
		super(/* isNetwork= */ true);
	}

	@Override
	public long open(DataSpec dataSpec) throws IOException {
		uri = dataSpec.uri;
		transferInitializing(dataSpec);
		opened = true;
		data = new byte[DEFAULT_MAX_PACKET_SIZE];

		transferStarted(dataSpec);
		return C.LENGTH_UNSET; // Return LENGTH_UNSET for live streams
	}

	@Override
	public int read(byte[] buffer, int offset, int length) throws IOException {
		if (length == 0) {
			return 0;
		}

		if (packetRemaining == 0) {
			// We've read all of the data from the current packet. Get another.
			data = RustService.helloByte(DEFAULT_MAX_PACKET_SIZE);

			packetRemaining = data.length;
			bytesTransferred(packetRemaining);
		}

		int packetOffset = data.length - packetRemaining;
		int bytesToRead = min(packetRemaining, length);
		System.arraycopy(data, packetOffset, buffer, offset, bytesToRead);
		packetRemaining -= bytesToRead;
		return bytesToRead;
	}

	@Override
	@Nullable
	public Uri getUri() {
		return uri;
	}

	@Override
	public void close() throws IOException {
		if (opened) {
			opened = false;
			transferEnded();
		}
		uri = null;
	}
}
