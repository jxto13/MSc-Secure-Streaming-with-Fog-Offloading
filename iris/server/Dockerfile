# Use the official Rust image from Docker Hub
FROM rust:latest

# Update and install dependencies
RUN apt update && apt install -y protobuf-compiler

# Create a user with UID/GID 1000 (matches typical host user)
RUN groupadd -g 1000 appgroup && \
    useradd -m -u 1000 -g 1000 -s /bin/bash appuser

# Create and set the working directory in the container
WORKDIR /home/appuser/server

# Copy shared libraries, proto files, and server code
COPY shared-lib ../shared-lib
COPY proto ../proto
COPY server .

# Set proper permissions for appuser
RUN chown -R appuser:appgroup /home/appuser

# Switch to non-root user
USER appuser

# Build the binary in release mode
RUN cargo build --release

# Use the precompiled binary directly
CMD ["./target/release/streaming_server", "--config", "config.toml"]
