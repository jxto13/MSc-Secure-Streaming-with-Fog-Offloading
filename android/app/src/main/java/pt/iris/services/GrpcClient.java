package pt.iris.services;
/*
import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import helloworld.GreeterGrpc;
import helloworld.Helloworld.HelloReply;
import helloworld.Helloworld.HelloRequest;

*//**
 * gRPC client for interacting with the server.
 *//*
public class GrpcClient implements AutoCloseable {
	*//**
	 * The managed channel for gRPC communication.
	 *//*
	private final ManagedChannel channel;

	*//**
	 * The blocking stub for making synchronous gRPC calls.
	 *//*
	private final GreeterGrpc.GreeterBlockingStub blockingStub;

	*//**
	 * Constructor for the GrpcClient class.
	 *
	 * @param host The host to connect to for gRPC communication.
	 * @param port The port on which the gRPC server is listening.
	 *//*
	public GrpcClient(String host, int port) {
		// Create a managed channel using the given host and port
		channel = ManagedChannelBuilder.forAddress(host, port)
				.usePlaintext()
				.build();

		// Create a blocking stub using the GreeterGrpc class and the created channel
		blockingStub = GreeterGrpc.newBlockingStub(channel);
	}

	*//**
	 * Sends a "Hello" request and receives the response.
	 *
	 * @param name The name to include in the "Hello" request.
	 * @return The message received in the response.
	 *//*
	public String sayHello(String name) {
		// Build a HelloRequest with the provided name
		HelloRequest request = HelloRequest.newBuilder().setName(name).build();

		// Use the blocking stub to send the request and receive the response
		HelloReply reply = blockingStub.sayHello(request);

		// Return the message from the received reply
		return reply.getMessage();
	}

	*//**
	 * Closes the managed channel to release resources.
	 *//*
	@Override
	public void close() {
		// Check if the channel is not null and has not been shut down
		if (channel != null && !channel.isShutdown()) {
			// Shut down the channel to release resources
			channel.shutdown();
		}
	}
}*/
