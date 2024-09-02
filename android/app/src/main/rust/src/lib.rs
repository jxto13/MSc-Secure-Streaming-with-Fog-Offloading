#[allow(non_snake_case)]
pub mod android {
    use std::thread;
    
    use android_logger::{self, init_once};
    use jni::objects::{JClass, JString};
    use jni::JNIEnv;
    use log::{error, info};

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

    
        thread::spawn( || {
            // A token to allow us to identify which event is for the `UdpSocket`.
            const SERVER: Token = Token(0);

            use mio::{Events, Interest, Poll, Token};
            use mio::net::UdpSocket;
            use std::io;

            let addr = "0.0.0.0:0".parse().unwrap();
            let mut socket = UdpSocket::bind(addr).unwrap();

            // Create a poll instance.
            let mut poll = Poll::new().unwrap();

            // Register the socket with the poll instance.
            poll.registry()
                .register(&mut socket, SERVER, Interest::WRITABLE).unwrap();

            // Create storage for events.
            let mut events = Events::with_capacity(1024);

            let receiver = iris_client::start().unwrap();
            
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
                                Err(err) => {
                                    // Channel is closed, handle the error or break out of the loop.
                                    error!("Data channel closed, exiting loop: {}", err);
                                    break; // Exit the loop if the channel is closed
                                }
                            }
                        }
                        // We don't expect any events with tokens other than those we provided.
                        _ => unreachable!(),
                    }
                }
            }
        });
    }
}
