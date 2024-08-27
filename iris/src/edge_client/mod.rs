pub mod core;
pub mod decoder;
pub mod receiver;
pub mod session;
pub mod utils;

pub mod proto {
    tonic::include_proto!("streaming_server");
    tonic::include_proto!("edge_client");
}