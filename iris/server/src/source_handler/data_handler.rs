use crossbeam_channel::{unbounded, Sender};
use log::info;


// Importing necessary modules and functions from other parts of the project.
use super::udp_source::read;
use super::encoder;

// Define an enum for different types of data sources. Currently, it only supports file-based sources.
pub enum DataSource {
    UDP(String), // A variant representing a file data source with the file path as a string.
    // Add more variants here if needed, e.g., Network(String), etc.
}

// Define a struct for handling data. This struct holds a data source and a sender channel.
pub struct DataHandler {
    source: DataSource, // The source of the data, which is of type DataSource.
    multicast_sender: Sender<Vec<u8>>, // A sender channel to transmit data.
}

// Implement methods for the DataHandler struct.
impl DataHandler {
    /// Constructor method to create a new DataHandler instance.
    ///
    /// # Arguments
    /// * `source` - The source of the data, specified as a DataSource enum.
    /// * `tx` - A Sender channel for transmitting the data.
    pub fn new(source: DataSource, multicast_sender: Sender<Vec<u8>>) -> Self {
        DataHandler { source, multicast_sender } // Initialize DataHandler with the provided data source and sender channel.
    }

    /// Method to start the data handling process.
    ///
    /// This method initializes the data reading process based on the data source,
    /// starts the encoder in a separate thread, and waits for the encoder to finish.
    pub fn start(&self) {
        
        // Create a new unbounded channel for internal communication.
        let (data_source_sender, data_source_receiver) = unbounded();

        // Match on the data source type and read the data accordingly.
        match &self.source {
            DataSource::UDP(ip_source) => read(ip_source.to_string(), data_source_sender).unwrap(),
            // Handle other data sources here, e.g., Network(String) => read_network_and_send(url, data_source_sender),
        }

        // Start the encoder
        // The encoder processes the received data and sends it using the cloned sender channel.
        encoder::start(data_source_receiver, self.multicast_sender.clone()); // Start the encoder with the receiver channel and the cloned sender channel.
        
        // Log a message indicating that the DataHandler has exited without errors.
        info!("DataHandler exit without errors");
    }
}
