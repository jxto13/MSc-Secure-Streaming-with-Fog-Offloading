# Use the official Rust image from Docker Hub
FROM rust:latest

# Create and set the working directory in the container
WORKDIR /usr/src/server

# Update and install dependencies
RUN apt update && apt install -y protobuf-compiler

# Copy shared libraries, proto files, and server code
COPY shared-lib ../shared-lib
COPY proto ../proto
COPY server .

# Build the binary in release mode
RUN cargo build --release

# Use the precompiled binary directly
CMD ["./target/release/streaming_server", "--config", "config.toml"]
