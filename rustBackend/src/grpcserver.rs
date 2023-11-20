use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use crate::service::rperf_start;

pub mod hello_world {
    tonic::include_proto!("helloworld"); // The string specified here must match the proto package name
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<HelloReply>, Status> { // Return an instance of type HelloReply

        log::set_max_level(log::LevelFilter::Warn);
        println!("Got a request: {:?}", request);

        let original_string = "rperf --client --udp --bandwidth 1000000".to_string();

        // Simulating the value from request.into_inner().name
        let client_name = request.into_inner().name;

        // Find the index to insert the client name
        let index_to_insert = original_string.find("--client").unwrap() + "--client".len();

        // Construct the new string by inserting the client name
        let modified_string = format!(
            "{} {} {}",
            &original_string[..index_to_insert],
            client_name,
            &original_string[index_to_insert..]
        );

        log::info!("{}",modified_string);

        
        // rperf_start(&modified_string);
        //rperf_start(&modified_string);
        
        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", client_name).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}

#[tokio::main]
pub async fn start_grpc_server() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}