package pt.iris.network.mesh;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.SelectionKey;
import java.nio.channels.Selector;
import java.nio.channels.ServerSocketChannel;
import java.nio.channels.SocketChannel;
import java.util.Iterator;
import java.util.Set;

public class PingServer implements Runnable {
	private static final int PORT = 8080;
	private boolean isRunning = true;

	@Override
	public void run() {
		try {
			ServerSocketChannel serverSocketChannel = ServerSocketChannel.open();
			serverSocketChannel.bind(new InetSocketAddress(PORT));
			serverSocketChannel.configureBlocking(false); // Set non-blocking mode

			Selector selector = Selector.open();
			serverSocketChannel.register(selector, SelectionKey.OP_ACCEPT);

			System.out.println("Server started on port " + PORT);

			while (isRunning) {
				selector.select(); // Blocking call, waits for events

				Set<SelectionKey> selectedKeys = selector.selectedKeys();
				Iterator<SelectionKey> keyIterator = selectedKeys.iterator();

				while (keyIterator.hasNext()) {
					SelectionKey key = keyIterator.next();
					keyIterator.remove();

					if (key.isAcceptable()) {
						ServerSocketChannel channel = (ServerSocketChannel) key.channel();
						SocketChannel clientChannel = channel.accept();
						clientChannel.configureBlocking(false);
						clientChannel.register(selector, SelectionKey.OP_READ);
						System.out.println("New client connected: " + clientChannel.getRemoteAddress());
					} else if (key.isReadable()) {
						SocketChannel clientChannel = (SocketChannel) key.channel();
						ByteBuffer buffer = ByteBuffer.allocate(1024);
						int bytesRead = clientChannel.read(buffer);
						if (bytesRead == -1) {
							clientChannel.close();
							continue;
						}
						buffer.flip();
						byte[] data = new byte[buffer.limit()];
						buffer.get(data);
						System.out.println("Received from client: " + new String(data));

						// Echo back to the client
						clientChannel.write(ByteBuffer.wrap("Pong!\n".getBytes()));
					}
				}
			}
		} catch (IOException e) {
			e.printStackTrace();
		}
	}


}
