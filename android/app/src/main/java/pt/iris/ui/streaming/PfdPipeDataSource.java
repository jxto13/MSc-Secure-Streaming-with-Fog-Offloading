package pt.iris.ui.streaming;

import android.net.Uri;
import android.os.ParcelFileDescriptor;
import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.annotation.OptIn;
import androidx.media3.common.C;
import androidx.media3.common.util.UnstableApi;
import androidx.media3.datasource.DataSource;
import androidx.media3.datasource.DataSpec;
import androidx.media3.datasource.TransferListener;

import java.io.FileInputStream;
import java.io.IOException;

@OptIn(markerClass = UnstableApi.class)
public class PfdPipeDataSource implements DataSource {
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
