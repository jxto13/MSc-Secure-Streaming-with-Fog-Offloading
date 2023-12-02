use tonic::service;
use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

use log::{debug, warn, error};


extern crate rperf;
extern crate tokio;

pub mod hello_world {
    tonic::include_proto!("helloworld"); // The string specified here must match the proto package name
}

#[derive(Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<HelloReply>, Status> { // Return an instance of type HelloReply

        //println!("Got a request: {:?}", request);

  
        // // Run the synchronous function in a tokio::task::spawn_blocking block
        // let name = request.into_inner().name.clone(); // Clone the name for the async block
        // // Spawn a blocking task to run the synchronous function
        // tokio::task::spawn_blocking(move || {
        //     // Run the synchronous function in a tokio::task::spawn_blocking block
        //     rperf::start_client(&name);
        // })
        // .await
        // .map_err(|err| {
        //     // Handle errors from the blocking task
        //     error!("Error in start_client: {:?}", err);
        //     Status::internal("Internal server error")
        // })?;

        let original_string = "rperf --client --bandwidth 1000000".to_string();

        // Simulating the value from request.into_inner().name
        let client_name = request.into_inner().name;
        let client_name2 = client_name.clone();

        // Find the index to insert the client name
        let index_to_insert = original_string.find("--client").unwrap() + "--client".len();

        // Construct the new string by inserting the client name
        let modified_string = format!(
            "{} {} {}",
            &original_string[..index_to_insert],
            client_name,
            &original_string[index_to_insert..]
        );

        // Spawn a blocking task to run the synchronous function
        // tokio::task::spawn_blocking(move || {
        //     // Run the synchronous function in a tokio::task::spawn_blocking block
        //     rperf::client::execute(crate::service::parse_args(&modified_string));
        // })
        // .await
        // .map_err(|err| {
        //     // Handle errors from the blocking task
        //     error!("Error in start_client: {:?}", err);
        //     Status::internal("Internal server error")
        // })?;

        //rperf::client::execute(crate::service::parse_args(&modified_string));

        // my_network_library::client::start_client(&client_name2);

               
        let reply = hello_world::HelloReply {
            message: format!("{}","ok").into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}

#[tokio::main]
pub async fn start_grpc_server() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "error");
    
    let addr = "[::]:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}