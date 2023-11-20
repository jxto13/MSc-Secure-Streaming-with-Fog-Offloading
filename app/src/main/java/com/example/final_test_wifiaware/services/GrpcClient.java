
package com.example.final_test_wifiaware.services;

import io.grpc.ManagedChannel;
import io.grpc.ManagedChannelBuilder;
import helloworld.GreeterGrpc;
import helloworld.Helloworld.HelloReply;
import helloworld.Helloworld.HelloRequest;

public class GrpcClient {
	private final GreeterGrpc.GreeterBlockingStub blockingStub;

	public GrpcClient(String host, int port) {
		ManagedChannel channel = ManagedChannelBuilder.forAddress(host, port)
				.usePlaintext()
				.build();

		blockingStub = GreeterGrpc.newBlockingStub(channel);
	}

	public String sayHello(String name) {
		HelloRequest request = HelloRequest.newBuilder().setName(name).build();
		HelloReply reply = blockingStub.sayHello(request);
		return reply.getMessage();
	}
}

