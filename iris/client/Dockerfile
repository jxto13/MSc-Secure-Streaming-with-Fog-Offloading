# Use the official Rust image from Docker Hub
FROM rust:latest

# Create and set the working directory in the container
WORKDIR /usr/iris/client

# Update and install dependencies
RUN apt update && apt install -y protobuf-compiler

# Copy shared libraries, proto files, and server code
COPY shared-lib ../shared-lib
COPY proto ../proto
COPY client .

# Copy the server public certificate to the expected location to compile the client
COPY server/certs/ca_certificate.pem ../server/certs/ca_certificate.pem

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
