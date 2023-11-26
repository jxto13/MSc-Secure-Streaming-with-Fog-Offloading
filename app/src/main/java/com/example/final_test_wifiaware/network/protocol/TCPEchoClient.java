package com.example.final_test_wifiaware.network.protocol;

import android.net.Network;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintWriter;
import java.net.Inet6Address;
import java.net.Socket;

public class TCPEchoClient extends Thread {

	private Inet6Address peerIpv6;
	private int peerPort;
	private Network network;

	public TCPEchoClient(Network network, Inet6Address serverAddress, int serverPort) {
		this.peerIpv6 = serverAddress;
		this.peerPort = serverPort;
		this.network = network;
	}

	@Override
	public void run() {
		try {
			while (true) {
				try (
						Socket socket = network.getSocketFactory().createSocket(peerIpv6, peerPort);
						PrintWriter out = new PrintWriter(socket.getOutputStream(), true);
						BufferedReader in = new BufferedReader(new InputStreamReader(socket.getInputStream()))
				) {
					// Send message to server
					out.println("Hello, server!");

					// Receive and print the echoed message
					String response = in.readLine();
					//System.out.println("Server response: " + response);
				} catch (IOException e) {
					e.printStackTrace();
				}

				// Wait for a while before the next iteration
				Thread.sleep(5000); // 5 seconds
			}
		} catch (InterruptedException e) {
			e.printStackTrace();
		}
	}


}
