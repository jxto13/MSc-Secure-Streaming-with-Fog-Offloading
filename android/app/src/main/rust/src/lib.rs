#[allow(non_snake_case)]
pub mod android {
    use std::io::Write;
    use std::os::fd::{FromRawFd, RawFd};
    use std::sync::{Arc, RwLock};
    use std::thread;

    use jni::objects::{JClass, JString};
    use jni::sys::jint;
    use jni::JNIEnv;
    use log::info;
    use once_cell::sync::OnceCell;
    use std::sync::Once;

    use iris_client::mesh_networking::Node;

    // Static variable to hold the iris client instance
    static CLIENT: OnceCell<Arc<RwLock<Option<iris_client::Client>>>> = OnceCell::new();

    // Thread handles to keep track of multiple spawned threads
    static THREAD_HANDLES: OnceCell<Arc<RwLock<Vec<std::thread::JoinHandle<()>>>>> =
        OnceCell::new();

    /// Initializes the logging system for Android.
    static INIT: Once = Once::new();

    static MESH_NETWORK_NODE: OnceCell<Arc<Node>> = OnceCell::new();

    // Initialize the mesh networking module
    pub fn init_mesh_networking() {
        let node = Node::new();
        if MESH_NETWORK_NODE.set(node).is_err() {
            info!("Mesh networking node already initialized");
        } else {
            info!("Mesh networking node created successfully");
        }
    }

    /// Initializes the logging system and panic hook for Android.
    fn init_logging() {
        INIT.call_once(|| {
            // Initialize Android logger
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Trace)
                    .with_tag("IrisClient"),
            );

            // Set up panic hook to log panics to Android logger
            std::panic::set_hook(Box::new(|panic_info| {
                let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                    s
                } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                    s.as_str()
                } else {
                    "Unknown panic payload"
                };

                let location = if let Some(location) = panic_info.location() {
                    format!(" at {}:{}:{}", location.file(), location.line(), location.column())
                } else {
                    String::new()
                };

                log::error!("PANIC: {}{}", payload, location);

                // Try to get thread info
                let binding = std::thread::current();
                let thread_name = binding.name().unwrap_or("unnamed");
                log::error!("Panic occurred in thread: {}", thread_name);

                // Log environment variable hint for backtrace
                log::error!("To enable backtrace, set RUST_BACKTRACE=1 environment variable before starting the application");
            }));

            log::info!("IrisClient logging and panic handling initialized for Android");            
        });
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startService(
        _env: JNIEnv,
        _class: JClass,
    ) {
        // Initialize the mesh networking node and logging
        init_logging();
        init_mesh_networking();
    }

    /// Starts the mesh networking node and its tasks.
    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startClient(
        mut env: JNIEnv,
        _class: JClass,
        fd: jint,
        stream_url: JString,
    ) -> jni::sys::jboolean {
        info!("Rust client is starting...");

        let fd = fd as RawFd;

        // Convert JString to Rust String
        let url_str: String = match env.get_string(&stream_url) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert stream URL from Java: {}", e);
                return jni::sys::JNI_FALSE;
            }
        };

        info!("Starting client with URL: {}", url_str);

        // Try to start the client and send result
        let mut client = iris_client::Client::new();

        match client.start(&url_str) {
            Ok(data_receiver) => {
                info!("Successfully connected to server");

                // Initialize CLIENT static if not already done
                if CLIENT.get().is_none() {
                    let _ = CLIENT.set(Arc::new(RwLock::new(None)));
                }
                if THREAD_HANDLES.get().is_none() {
                    let _ = THREAD_HANDLES.set(Arc::new(RwLock::new(Vec::new())));
                }

                // Store the client
                if let Some(client_holder) = CLIENT.get() {
                    if let Ok(mut guard) = client_holder.write() {
                        *guard = Some(client);
                    }
                }

                let handle = thread::spawn(move || {
                    // SAFETY: fd must be valid and owned by Java
                    let file = unsafe { std::fs::File::from_raw_fd(fd) };

                    // Clone so Rust can write, but Java still owns fd
                    let mut writer = match file.try_clone() {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Failed to clone fd: {:?}", e);
                            return;
                        }
                    };
                    // Don't close the original fd owned by Java
                    std::mem::forget(file); // avoid closing fd

                    // Loop over incoming data
                    for item in data_receiver {
                        if let Err(e) = writer.write_all(&item) {
                            eprintln!("Write error: {:?}", e);
                            break;
                        }
                    }
                    log::error!("Client stream ended for URL: {}", url_str);
                });

                // Store the thread handle in the vector
                if let Some(handles_holder) = THREAD_HANDLES.get() {
                    if let Ok(mut guard) = handles_holder.write() {
                        guard.push(handle);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to start iris client with URL {}: {:?}", url_str, e);
            }
        }

        // Get safe holder of client
        if let Some(client_holder) = CLIENT.get() {
            if let Ok(guard) = client_holder.read() {
                if let Some(client) = guard.as_ref() {
                    let retransmission_manager = client.get_retransmission_manager();

                    // Get the mesh networking node
                    let node = match MESH_NETWORK_NODE.get() {
                        Some(node) => node.clone(),
                        None => {
                            log::error!("Mesh networking node is not initialized");
                            return jni::sys::JNI_FALSE;
                        }
                    };

                    // Get the IPC channels from Node to RetransmissionManager
                    let ipc_channels = node.get_ipc_channels();

                    let handle = thread::spawn({
                        let ipc_channels = Arc::clone(&ipc_channels);
                        move || {
                            // Correct channel mapping:
                            // - rx.0 is what retransmission manager uses to send TO mesh (mesh receives via rx.1)
                            // - tx.1 is what retransmission manager uses to receive FROM mesh (mesh sends via tx.0)
                            retransmission_manager
                                .middleware(ipc_channels.rx.0.clone(), &ipc_channels.tx.1);
                        }
                    });

                    // Store the thread handle in the vector
                    if let Some(handles_holder) = THREAD_HANDLES.get() {
                        if let Ok(mut guard) = handles_holder.write() {
                            guard.push(handle);
                        }
                    }
                } else {
                    log::error!("Iris client is not initialized");
                }
            } else {
                log::error!("Failed to acquire read lock on client");
            }
        } else {
            log::error!("Iris client holder is not initialized");
        }

        jni::sys::JNI_TRUE
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_stopClient(
        _env: JNIEnv,
        _class: JClass,
    ) {
        log::info!("Stopping iris client...");

        // Shutdown the client first
        if let Some(client_holder) = CLIENT.get() {
            if let Ok(mut guard) = client_holder.write() {
                if let Some(client) = guard.take() {
                    client.shutdown();
                    log::info!("Iris client shutdown requested");
                }
            }
        }

        // Wait for all threads to finish
        if let Some(handles_holder) = THREAD_HANDLES.get() {
            if let Ok(mut guard) = handles_holder.write() {
                while let Some(handle) = guard.pop() {
                    match handle.join() {
                        Ok(_) => log::info!("Thread joined successfully"),
                        Err(e) => log::error!("Error joining thread: {:?}", e),
                    }
                }
            }
        }

        log::info!("Iris client shutdown complete");
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_addSocket(
        mut env: JNIEnv,
        _class: JClass,
        ip_address: JString,
    ) {
        // Convert JString to Rust String
        let ip_str: String = match env.get_string(&ip_address) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert IP address from Java: {}", e);
                return;
            }
        };

        log::info!("Adding socket for IP: {}", ip_str);

        // Get node instance
        let node = match MESH_NETWORK_NODE.get() {
            Some(node) => node.clone(),
            None => {
                log::error!("Mesh networking node is not initialized");
                return;
            }
        };

        // Add socket to the node
        match node.add_socket_sync(&ip_str) {
            Ok(_) => log::info!("Successfully added socket for IP: {}", ip_str),
            Err(e) => log::error!("Failed to add socket for IP {}: {:?}", ip_str, e),
        };
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_getPacketLossPercentage(
        _env: JNIEnv,
        _class: JClass,
    ) -> f64 {
        // SAFETY: CLIENT is a static variable, so we can safely access it
        if let Some(client_holder) = CLIENT.get() {
            if let Ok(guard) = client_holder.read() {
                if let Some(client) = guard.as_ref() {
                    // Get the packet loss percentage from the client
                    client.get_packet_loss_percentage()
                } else {
                    log::error!("Iris client is not initialized");
                    0.0
                }
            } else {
                log::error!("Failed to acquire read lock on client");
                0.0
            }
        } else {
            0.0
        }
    }
}
