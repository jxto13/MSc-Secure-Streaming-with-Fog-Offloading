#[allow(non_snake_case)]
pub mod android {
    use std::io::Write;
    use std::os::fd::{FromRawFd, RawFd};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, RwLock};
    use std::thread;
    use std::time::Duration;

    use jni::objects::{JClass, JString};
    use jni::sys::{jboolean, jint, JNI_FALSE, JNI_TRUE};
    use jni::JNIEnv;
    use log::info;
    use once_cell::sync::OnceCell;
    use std::sync::Once;

    use iris_client::mesh_networking::Node;

    // Global Iris client
    static CLIENT: OnceCell<Arc<RwLock<Option<iris_client::Client>>>> = OnceCell::new();

    // Android logging init
    static INIT: Once = Once::new();

    // Mesh node
    static MESH_NETWORK_NODE: OnceCell<Arc<Node>> = OnceCell::new();

    // Global cancel flag
    static SHUTDOWN: OnceCell<Arc<AtomicBool>> = OnceCell::new();

    pub fn init_mesh_networking() {
        let node = Node::new();
        if MESH_NETWORK_NODE.set(node).is_err() {
            info!("Mesh networking node already initialized");
        } else {
            info!("Mesh networking node created successfully");
        }
    }

    fn init_logging() {
        INIT.call_once(|| {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Trace)
                    .with_tag("IrisClient"),
            );

            std::panic::set_hook(Box::new(|panic_info| {
                let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                    s
                } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                    s.as_str()
                } else {
                    "Unknown panic payload"
                };

                let location = if let Some(location) = panic_info.location() {
                    format!(
                        " at {}:{}:{}",
                        location.file(),
                        location.line(),
                        location.column()
                    )
                } else {
                    String::new()
                };

                log::error!("PANIC: {}{}", payload, location);

                let binding = std::thread::current();
                let thread_name = binding.name().unwrap_or("unnamed");
                log::error!("Panic occurred in thread: {}", thread_name);

                log::error!(
                    "To enable backtrace, set RUST_BACKTRACE=1 before starting the application"
                );
            }));

            log::info!("IrisClient logging and panic handling initialized for Android");
        });
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startService(
        _env: JNIEnv,
        _class: JClass,
    ) {
        init_logging();
        init_mesh_networking();

        let _ = CLIENT.set(Arc::new(RwLock::new(None)));
        let _ = SHUTDOWN.set(Arc::new(AtomicBool::new(false)));

        if let Some(tok) = SHUTDOWN.get() {
            tok.store(false, Ordering::SeqCst);
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startClient(
        mut env: JNIEnv,
        _class: JClass,
        fd: jint,
        stream_url: JString,
    ) -> jboolean {
        info!("Rust client is starting...");

        // Refuse double start
        if let Some(client_holder) = CLIENT.get() {
            if let Ok(g) = client_holder.read() {
                if g.is_some() {
                    log::error!("Client already running; stop it before starting again");
                    return JNI_FALSE;
                }
            }
        }

        // Ensure globals
        let client_holder = CLIENT.get().cloned().unwrap_or_else(|| {
            let _ = CLIENT.set(Arc::new(RwLock::new(None)));
            CLIENT.get().unwrap().clone()
        });

        let shutdown = SHUTDOWN.get().cloned().unwrap_or_else(|| {
            let _ = SHUTDOWN.set(Arc::new(AtomicBool::new(false)));
            SHUTDOWN.get().unwrap().clone()
        });
        shutdown.store(false, Ordering::SeqCst);

        // Inputs
        let fd = fd as RawFd;
        let url_str: String = match env.get_string(&stream_url) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert stream URL from Java: {}", e);
                return JNI_FALSE;
            }
        };
        info!("Starting client with URL: {}", url_str);

        // Start client
        let mut client = iris_client::Client::new();
        let data_receiver = match client.start(&url_str) {
            Ok(rx) => {
                info!("Successfully connected to server");
                rx
            }
            Err(e) => {
                log::error!("Failed to start iris client with URL {}: {:?}", url_str, e);
                return JNI_FALSE;
            }
        };

        // Store client
        if let Ok(mut guard) = client_holder.write() {
            *guard = Some(client);
        } else {
            log::error!("Failed to acquire write lock on client holder");
            return JNI_FALSE;
        }

        // Data forwarding thread (detached)
        {
            let shutdown_clone = shutdown.clone();
            let url_for_log = url_str.clone();

            let _ = thread::Builder::new()
                .name("iris_data_forward".to_string())
                .spawn(move || {
                    // SAFETY: fd is provided by Java. Avoid closing it on this side.
                    let file = unsafe { std::fs::File::from_raw_fd(fd) };

                    let mut writer = match file.try_clone() {
                        Ok(f) => f,
                        Err(e) => {
                            log::error!("Failed to clone fd: {:?}", e);
                            // Prevent closing Java's fd
                            std::mem::forget(file);
                            return;
                        }
                    };
                    // Do not close Java-owned fd
                    std::mem::forget(file);

                    for item in data_receiver {
                        if shutdown_clone.load(Ordering::Relaxed) {
                            break;
                        }
                        if let Err(e) = writer.write_all(&item) {
                            log::error!("Write error: {:?}", e);
                            break;
                        }
                    }

                    let _ = writer.flush();
                    log::info!("Client stream ended for URL: {}", url_for_log);
                });
        }

        // Retransmission middleware thread (detached)
        {
            let (retransmission_manager, ipc_channels) = {
                let guard = client_holder.read();
                if let Ok(g) = guard {
                    if let Some(cl) = g.as_ref() {
                        let rm = cl.get_retransmission_manager();
                        let node = match MESH_NETWORK_NODE.get() {
                            Some(node) => node.clone(),
                            None => {
                                log::error!("Mesh networking node is not initialized");
                                return JNI_FALSE;
                            }
                        };
                        let ipc = node.get_ipc_channels();
                        (rm, ipc)
                    } else {
                        log::error!("Iris client is not initialized after start");
                        return JNI_FALSE;
                    }
                } else {
                    log::error!("Failed to acquire read lock on client");
                    return JNI_FALSE;
                }
            };

            let shutdown_clone = shutdown.clone();

            let _ = thread::Builder::new()
                .name("iris_middleware".to_string())
                .spawn(move || {
                    // Cancel-aware wrapper around blocking middleware
                    let rx_to_mesh = ipc_channels.rx.0.clone();
                    let from_mesh_rx_ref = ipc_channels.tx.1.clone();

                    if shutdown_clone.load(Ordering::Relaxed) {
                        drop(rx_to_mesh);
                        return;
                    }

                    let watcher_cancel = shutdown_clone.clone();
                    let watcher_tx = rx_to_mesh.clone();
                    let _watcher = thread::spawn(move || {
                        while !watcher_cancel.load(Ordering::Relaxed) {
                            thread::sleep(Duration::from_millis(50));
                        }
                        drop(watcher_tx);
                    });

                    retransmission_manager.middleware(rx_to_mesh, &from_mesh_rx_ref);
                    log::info!("middleware exited");
                });
        }

        JNI_TRUE
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_stopClient(
        _env: JNIEnv,
        _class: JClass,
    ) -> jboolean {
        log::info!("Stopping iris client...");

        // Trip cancel so outer loops break
        if let Some(tok) = SHUTDOWN.get() {
            tok.store(true, Ordering::SeqCst);
        }

        // Ask client to shut down, which closes channels and unblocks threads
        if let Some(client_holder) = CLIENT.get() {
            if let Ok(mut guard) = client_holder.write() {
                if let Some(client) = guard.take() {
                    client.shutdown();
                    log::info!("Iris client shutdown requested");
                }
            } else {
                log::error!("Failed to acquire write lock on client during shutdown");
            }
        }

        // Keep SHUTDOWN=true long enough for detached threads to notice.
        // Do not immediately reset to false here. Reset on next start.
        log::info!("Iris client shutdown complete");
        JNI_TRUE
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_addSocket(
        mut env: JNIEnv,
        _class: JClass,
        ip_address: JString,
    ) {
        let ip_str: String = match env.get_string(&ip_address) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert IP address from Java: {}", e);
                return;
            }
        };

        log::info!("Adding socket for IP: {}", ip_str);

        let node = match MESH_NETWORK_NODE.get() {
            Some(node) => node.clone(),
            None => {
                log::error!("Mesh networking node is not initialized");
                return;
            }
        };

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
        if let Some(client_holder) = CLIENT.get() {
            if let Ok(guard) = client_holder.read() {
                if let Some(client) = guard.as_ref() {
                    client.get_packet_loss_percentage()
                } else {
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
