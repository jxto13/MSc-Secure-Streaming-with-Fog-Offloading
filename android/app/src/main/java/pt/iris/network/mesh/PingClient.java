package pt.iris.network.mesh;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.Selector;
import java.nio.channels.SocketChannel;
import java.util.ArrayDeque;
import java.util.Queue;
import java.net.InetAddress;

public class PingClient implements Runnable {
	private InetAddress serverHost;
	private int serverPort;
	private Queue<String> messagesToSend = new ArrayDeque<>();
	private volatile boolean running = true; // Flag to control the execution

	public PingClient(InetAddress serverHost, int serverPort) {
		this.serverHost = serverHost;
		this.serverPort = serverPort;
	}

	public void addMessage(String message) {
		messagesToSend.add(message);
	}

	public void stop() {
		running = false; // Set the flag to stop execution
	}

	@Override
	public void run() {
		try (SocketChannel socketChannel = SocketChannel.open()) {
			socketChannel.connect(new InetSocketAddress(serverHost, serverPort));
			System.out.println("Connected to server " + serverHost.getHostAddress() + ":" + serverPort);

			Selector selector = Selector.open();
			socketChannel.configureBlocking(false);
			socketChannel.register(selector, socketChannel.validOps());

			ByteBuffer buffer = ByteBuffer.allocate(1024);
			while (running) { // Check the flag to continue or stop execution
				if (!messagesToSend.isEmpty()) {
					String message = messagesToSend.poll();
					socketChannel.write(ByteBuffer.wrap(message.getBytes()));
					System.out.println("Sent to server " + serverHost.getHostAddress() + ":" + serverPort + ": " + message);
				}

				selector.select(); // Wait for events

				int bytesRead = socketChannel.read(buffer);
				if (bytesRead == -1) {
					System.out.println("Connection closed by server " + serverHost.getHostAddress() + ":" + serverPort);
					break;
				} else if (bytesRead > 0) {
					buffer.flip();
					byte[] responseData = new byte[bytesRead];
					buffer.get(responseData);
					String response = new String(responseData);
					System.out.println("Received from server " + serverHost.getHostAddress() + ":" + serverPort + ": " + response);
					buffer.clear();
				}
			}
		} catch (IOException e) {
			if (e instanceof java.net.ConnectException) {
				System.out.println("Connection aborted by server " + serverHost.getHostAddress() + ":" + serverPort);
			} else {
				e.printStackTrace();
			}
		} finally {
			stop(); // Ensure proper cleanup and exit
		}
	}

}



