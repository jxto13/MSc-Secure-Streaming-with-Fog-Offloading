package com.example.final_test_wifiaware.network.protocol;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintWriter;
import java.net.ServerSocket;
import java.net.Socket;

public class TCPEchoServer extends Thread {

	private ServerSocket serverSocket;

	public TCPEchoServer(int port) throws IOException {
		serverSocket = new ServerSocket(port);
	}

	@Override
	public void run() {
		try {
			while (true) {
				//System.out.println("Waiting for client on port " + serverSocket.getLocalPort() + "...");
				Socket clientSocket = serverSocket.accept();
				//System.out.println("Just connected to " + clientSocket.getRemoteSocketAddress());

				// Handle client connection in a new thread
				Thread clientHandlerThread = new Thread(new ClientHandler(clientSocket));
				clientHandlerThread.start();
			}
		} catch (IOException e) {
			e.printStackTrace();
		}
	}


	private static class ClientHandler implements Runnable {

		private Socket clientSocket;

		public ClientHandler(Socket clientSocket) {
			this.clientSocket = clientSocket;
		}

		@Override
		public void run() {
			try (
					BufferedReader in = new BufferedReader(new InputStreamReader(clientSocket.getInputStream()));
					PrintWriter out = new PrintWriter(clientSocket.getOutputStream(), true)
			) {
				String inputLine;

				while ((inputLine = in.readLine()) != null) {
					//System.out.println("Received from client: " + inputLine);
					out.println("Server echo: " + inputLine);
				}

				clientSocket.close();
			} catch (IOException e) {
				e.printStackTrace();
			}
		}
	}
}
