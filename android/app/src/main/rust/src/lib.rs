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

    fn init_logging() {
        use std::sync::Once;
        static INIT: Once = Once::new();

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
        }
    );
    }
    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startClient(
        mut env: JNIEnv,
        _class: JClass,
        fd: jint,
        stream_url: JString,
    ) -> jni::sys::jboolean {
        init_logging();

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
        let client_result = iris_client::start_client(&url_str);

        match client_result {
            Ok(client) => {
                info!("Successfully connected to server");

                thread::spawn(move || {
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
                    for item in client {
                        if let Err(e) = writer.write_all(&item) {
                            eprintln!("Write error: {:?}", e);
                            break;
                        }
                    }
                    log::error!("Client stream ended for URL: {}", url_str);
                });
            }
            Err(e) => {
                log::error!("Failed to start iris client with URL {}: {:?}", url_str, e);
                return jni::sys::JNI_FALSE;
            }
        }

        jni::sys::JNI_TRUE
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_stopClient(
        _env: JNIEnv,
        _class: JClass,
    ) {
        log::info!("Stopping iris client...");
        iris_client::shutdown_client();
        log::info!("Iris client shutdown requested");
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

        match iris_client::add_socket(&ip_str) {
            Ok(_) => log::info!("Successfully added socket for IP: {}", ip_str),
            Err(e) => log::error!("Failed to add socket for IP {}: {:?}", ip_str, e),
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_getPacketLossPercentage(
        _env: JNIEnv,
        _class: JClass,
    ) -> f64 {
        // TODO: Implement actual packet loss calculation from iris_client
        // For now, return a mock value that changes over time
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let variation = (now % 20) as f64 * 0.1; // 0.0 to 2.0% variation
        let result = 0.5 + variation; // Base 0.5% + variation
        log::trace!("Returning packet loss percentage: {:.2}%", result);
        result
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_getCurrentBitrateMbps(
        _env: JNIEnv,
        _class: JClass,
    ) -> f64 {
        // TODO: Implement actual bitrate calculation from iris_client
        // For now, return a mock value that simulates realistic bitrate
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let variation = (now % 30) as f64 * 0.5; // 0.0 to 15.0 Mbps variation
        let result = 25.0 + variation; // Base 25 Mbps + variation
        log::trace!("Returning current bitrate: {:.2} Mbps", result);
        result
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_getTotalDataTransferredMB(
        _env: JNIEnv,
        _class: JClass,
    ) -> i64 {
        // TODO: Implement actual data transfer tracking from iris_client
        // For now, return a mock value that increases over time
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let result = (now / 10) as i64; // Increases by 1 MB every 10 seconds
        log::trace!("Returning total data transferred: {} MB", result);
        result
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_getActiveConnectionCount(
        _env: JNIEnv,
        _class: JClass,
    ) -> i32 {
        // TODO: Implement actual connection count from iris_client
        // For now, return a mock value based on time
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let result = ((now % 5) + 1) as i32; // 1 to 5 connections
        log::trace!("Returning active connection count: {}", result);
        result
    }
}
