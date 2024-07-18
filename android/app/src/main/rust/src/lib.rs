use log::info;

#[allow(non_snake_case)]
pub mod android {
    use std::io::Write;
    use std::thread;
    use std::fs::OpenOptions;
    use std::path::Path;

    use android_logger::{self, init_once};
    use jni::objects::{JClass, JString};
    use jni::JNIEnv;
    use log::{error, info};
    
    
    use std::sync::Arc;
    use std::sync::Mutex;

    use iris::edge_client;

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startService(
        _: JNIEnv,
        _: JClass,
        _: JString,
    ) {
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("RustApp"),
        );

        // Log that the Rust app is starting
        info!("Rust app is starting...");
        
        // thread::spawn(||{
        //     // Start the Rust processing thread and receive the receiver
        //     if let Ok(receiver) = edge_client::core::start() {

        //         for item in receiver {
        //             // info!("{}", item.len());
        //         }

        //     } else {
        //         error!("Failed to start Rust processing thread");
        //     }
        // });
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_checkWritePermission(
        mut env: JNIEnv,
        _: JClass,
        jpath: JString,
    ) {
        // A token to allow us to identify which event is for the `UdpSocket`.
        const SERVER: Token = Token(0);

        use mio::{Events, Interest, Poll, Token};
        use mio::net::UdpSocket;
        use std::io;

        let addr = "127.0.0.1:0".parse().unwrap();
        let mut socket = UdpSocket::bind(addr).unwrap();

        // Create a poll instance.
        let mut poll = Poll::new().unwrap();

        // Register the socket with the poll instance.
        poll.registry()
            .register(&mut socket, SERVER, Interest::WRITABLE).unwrap();

        // Create storage for events.
        let mut events = Events::with_capacity(1024);

        let receiver = edge_client::core::start().unwrap();
        
        // Start an event loop.
        loop {
            // Poll Mio for events, blocking until we get an event.
            // Poll to check if we have events waiting for us.
            if let Err(err) = poll.poll(&mut events, None) {
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
            }

            // Process each event.
            for event in events.iter() {
                // We can use the token we previously provided to `register` to
                // determine for which socket the event is.
                match event.token() {
                    SERVER => {
                        // Check if there is data to be sent over multicast.
                        // If there's data available, send it over multicast.
                        match receiver.recv() {
                            Ok(data) => {
                                
                                // Send the data over multicast.
                                if let Err(e) = socket.send_to(&data, "127.0.0.1:12346".parse().unwrap()) {
                                    error!("Failed to send data over multicast: {}", e);
                                }
                            }
                            Err(_) => {
                                // Channel is closed, handle the error or break out of the loop.
                                error!("Data channel closed, exiting loop");
                                break; // Exit the loop if the channel is closed
                            }
                        }
                    }
                    // We don't expect any events with tokens other than those we provided.
                    _ => unreachable!(),
                }
            }
        }

    }

    // #[no_mangle]
    // pub extern "system" fn Java_pt_iris_services_RustService_checkWritePermission(
    //     mut env: JNIEnv,
    //     _: JClass,
    //     jpath: JString,
    // ) {

    //     // Convert the Java string to a Rust string
    //     let path: String = env
    //         .get_string(&jpath)
    //         .expect("Couldn't get Java string!")
    //         .into();

    //     // Check if the path is writable
    //     let path_obj = Path::new(&path);
    //     let result = OpenOptions::new()
    //         .write(true)
    //         // create(true)
    //         .open(path_obj);

    //     match result {
    //         Ok(mut file) => {
    //             info!("Path is writable: {}", path);
    //                 // Start the Rust processing thread and receive the receiver
    //                 if let Ok(receiver) = edge_client::core::start() {
    //                     for item in receiver {
    //                         // info!("{:?}",item);
    //                         // Write the item to the file
    //                         if let Err(e) = file.write_all(&item) {
    //                             error!("Failed to write to file: {}. Error: {}", path, e);
    //                         }
    //                     }
    //                 } else {
    //                     error!("Failed to start Rust processing thread");
    //                 }
    //             }
    //         Err(e) => {
    //             error!("Cannot write to path: {}. Error: {}", path, e)
    //         },
    //     };

    // }

    // Define a struct to hold shared state or callback function
    // struct AsyncFileWriter {
    //     // Example: You can use Arc<Mutex<T>> to share mutable state across threads safely
    //     // For simplicity, this example just uses a flag indicating success or failure
    //     success: bool,
    // }

    // impl AsyncFileWriter {
    //     // Example method to simulate asynchronous file writing
    //     fn start_writing_async(path: String) -> Arc<Mutex<Self>> {
    //         let state = Arc::new(Mutex::new(AsyncFileWriter {
    //             success: false,
    //         }));

    //         // Clone Arc for thread move
    //         let state_clone = Arc::clone(&state);

    //         // Spawn a new thread for asynchronous processing
    //         thread::spawn(move || {
    //             // Simulate writing to file asynchronously
    //             // Replace this with your actual logic
    //             if let Ok(receiver) = edge_client::core::start() {
    //                 for item in receiver {
    //                     if let Err(err) = write_to_file(&path, &item) {
    //                         error!("{}",err);
    //                     }
    //                 }
    //                 // If all writes are successful, set success flag to true
    //                 let mut state = state_clone.lock().unwrap();
    //                 state.success = true;
    //             } else {
    //                 // Handle thread start failure
    //                 let mut state = state_clone.lock().unwrap();
    //                 state.success = false;
    //             }
    //         });

    //         state
    //     }
    // }

    // #[no_mangle]
    // pub extern "system" fn Java_pt_iris_services_RustService_checkWritePermission(
    //     mut env: JNIEnv,
    //     _: JClass,
    //     jpath: JString,
    // ) {
    //     init_once(
    //         android_logger::Config::default()
    //             .with_max_level(log::LevelFilter::Info)
    //             .with_tag("RustApp"),
    //     );

    //     // Convert the Java string to a Rust string
    //     let path: String = env
    //         .get_string(&jpath)
    //         .expect("Couldn't get Java string!")
    //         .into();

    //     // Start asynchronous file writing
    //     let _state = AsyncFileWriter::start_writing_async(path);

    //     // Return from JNI call immediately
    //     info!("JNI call returned immediately. File writing is in progress asynchronously.");
    // }

    // // Example function to simulate writing to file
    // fn write_to_file(path: &str, data: &[u8]) -> Result<(), std::io::Error> {
    //     use std::fs::OpenOptions;
    //     use std::io::Write;
    //     use std::path::Path;

    //     let path_obj = Path::new(path);
    //     let mut file = OpenOptions::new()
    //         .write(true)
    //         .create(true)
    //         .open(path_obj)?;

    //     file.write_all(data)?;
    //     Ok(())
    // }

}
