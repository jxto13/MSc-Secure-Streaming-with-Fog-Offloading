#[derive(Debug, Default)]
pub struct RetransmissionManager {
    /// A list of all active retransmission requests.
    active_requests: VecDeque<RetransmissionRequest>,
}
impl RetransmissionManager {
    pub fn new() -> Self {
        Self {
            active_requests: VecDeque::new(),
        }
    }

    /// Adds a new retransmission request to the manager.
    pub fn add_request(&mut self, request: RetransmissionRequest) {
        self.active_requests.push_back(request);
    }

    /// Processes all active retransmission requests.
    pub fn process_requests(&mut self) {
        while let Some(request) = self.active_requests.pop_front() {
            // Process the request (e.g., send a retransmission request)
            // This is a placeholder for actual processing logic.
            println!("Processing request: {:?}", request);
        }
    }
}
