use chrono::Utc;
use tonic::{Request, Response, Status};

use crate::proto::{nts_service_server::NtsService, TimestampRequest, TimestampResponse};

// Define the service struct.
#[derive(Debug, Default)]
pub struct MyNtsService {}

// Implement the gRPC methods for the NtsService trait.
#[tonic::async_trait]
impl NtsService for MyNtsService {
    async fn get_current_timestamp(
        &self,
        _request: Request<TimestampRequest>,
    ) -> Result<Response<TimestampResponse>, Status> {
        // Get the current UTC time.
        let current_time = Utc::now().to_rfc3339();

        // Create a response with the current timestamp.
        let reply = TimestampResponse {
            timestamp: current_time,
        };

        // Return the response.
        Ok(Response::new(reply))
    }
}
