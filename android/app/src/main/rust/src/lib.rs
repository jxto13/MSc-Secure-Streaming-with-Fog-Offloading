#[allow(non_snake_case)]
pub mod android {
    use std::ffi::CString;
    use std::fs;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::thread;
    
    use android_logger::{self, init_once};
    use jni::objects::{JClass, JString};
    use jni::JNIEnv;
    use log::{error, info};
    use iris::edge_client;
    use libc;

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
        
        thread::spawn(move ||{
            // Start the Rust processing thread and receive the receiver
            if let Ok(receiver) = edge_client::core::start() {
                
                for item in receiver {
                    info!("{}",item.len());
                }
            } else {
                error!("Failed to start Rust processing thread");
            }
        });

        let path = "/data/data/pt.iris/cache/pipes/pipe1";
        

        // Create a named pipe (FIFO)
        if !Path::new(path).exists() {
            let fifo_mode = 0o700; // Set the desired permissions: prw---------
            fs::create_dir_all("/data/data/pt.iris/cache/pipes").unwrap(); // Ensure the directory exists

            // Convert the path to a CString to ensure it's null-terminated
            let c_path = CString::new(path).expect("CString::new failed");
            // Create the FIFO with the specified permissions
            unsafe {
                let result = libc::mkfifo(c_path.as_ptr(), fifo_mode);
                if result != 0 {
                    panic!("Failed to create FIFO");
                }
            }
            info!("FIFO created at {}", path);
        }
        
        

        // thread::spawn( || {
        //     // A token to allow us to identify which event is for the `UdpSocket`.
        //     const SERVER: Token = Token(0);

        //     use mio::{Events, Interest, Poll, Token};
        //     use mio::net::UdpSocket;
        //     use std::io;

        //     let addr = "0.0.0.0:0".parse().unwrap();
        //     let mut socket = UdpSocket::bind(addr).unwrap();

        //     // Create a poll instance.
        //     let mut poll = Poll::new().unwrap();

        //     // Register the socket with the poll instance.
        //     poll.registry()
        //         .register(&mut socket, SERVER, Interest::WRITABLE).unwrap();

        //     // Create storage for events.
        //     let mut events = Events::with_capacity(1024);

        //     let receiver = edge_client::core::start().unwrap();

        //     let mut buffer = Vec::new();
            
        //     // Start an event loop.
        //     loop {
        //         // Poll Mio for events, blocking until we get an event.
        //         // Poll to check if we have events waiting for us.
        //         if let Err(err) = poll.poll(&mut events, None) {
        //             if err.kind() == io::ErrorKind::Interrupted {
        //                 continue;
        //             }
        //         }

        //         // Process each event.
        //         for event in events.iter() {
        //             // We can use the token we previously provided to `register` to
        //             // determine for which socket the event is.
        //             match event.token() {
        //                 SERVER => {
        //                     // Check if there is data to be sent over multicast.
        //                     // If there's data available, send it over multicast.
        //                     match receiver.recv() {
        //                         Ok(mut data) => {
        //                             // Append new data to the buffer
        //                             buffer.append(&mut data);
                    
        //                             while buffer.len() >= 188 {
        //                                 // Split off 188 bytes from the buffer
        //                                 let chunk: Vec<u8> = buffer.drain(0..188).collect();
                    
        //                                 // Send the data over multicast.
        //                                 if let Err(e) = socket.send_to(&chunk, "127.0.0.1:12346".parse().unwrap()) {
        //                                     error!("Failed to send data over multicast: {}", e);
        //                                 }
                    
        //                                 if let Err(e) = socket.send_to(&chunk, "192.168.0.66:12346".parse().unwrap()) {
        //                                     error!("Failed to send data over multicast: {}", e);
        //                                 }
        //                             }
        //                         }
        //                         Err(_) => {
        //                             // Channel is closed, handle the error or break out of the loop.
        //                             error!("Data channel closed, exiting loop");
        //                             break; // Exit the loop if the channel is closed
        //                         }
        //                     }
        //                 }
        //                 // We don't expect any events with tokens other than those we provided.
        //                 _ => unreachable!(),
        //             }
        //         }
        //     }
        // });
    }
}
