FROM rust:latest

WORKDIR /usr/src/app

RUN apt update
RUN apt install -y protobuf-compiler

COPY . .

RUN cargo build --release

CMD cargo run --bin streaming_server -- --config config.toml