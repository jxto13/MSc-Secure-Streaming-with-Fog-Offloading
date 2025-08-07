# Use the official Rust image from Docker Hub
FROM rust:latest

# Update and install dependencies
RUN apt update && apt install -y protobuf-compiler


# Create a user with UID/GID 1000 (matches typical host user)
RUN useradd -m appuser

# Create and set the working directory in the container
WORKDIR /home/appuser/client

# Copy shared libraries, proto files, and server code
COPY shared-lib ../shared-lib
COPY proto ../proto
COPY client .

# Copy the server public certificate to the expected location to compile the client
COPY server/certs/ca_certificate.pem ../server/certs/ca_certificate.pem

# Set proper permissions for appuser
RUN chown -R appuser /home/appuser

# Switch to non-root user
USER appuser

# Build the binary in release mode
RUN cargo build --release --example headless

# Use the precompiled binary directly
CMD ["sh", "-c", "\
  if [ -z \"$SERVER_IP\" ]; then \
    echo 'ERROR: SERVER_IP environment variable is not set' >&2; \
    exit 1; \
  else \
    ./target/release/examples/headless --ip \"$SERVER_IP\"; \
  fi"]
