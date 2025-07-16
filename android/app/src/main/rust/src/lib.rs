#[allow(non_snake_case)]
pub mod android {
    use std::io::Write;
    use std::os::fd::{FromRawFd, RawFd};
    use std::thread;

    use android_logger::{self, init_once};
    use jni::objects::{JClass, JString};
    use jni::sys::jint;
    use jni::JNIEnv;
    use log::info;

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_startClient(
        mut env: JNIEnv,
        _class: JClass,
        fd: jint,
        stream_url: JString,
    ) {
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Trace)
                .with_tag("RustApp"),
        );

        info!("Rust client is starting...");

        let fd = fd as RawFd;

        // Convert JString to Rust String
        let url_str: String = match env.get_string(&stream_url) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert stream URL from Java: {}", e);
                return;
            }
        };

        info!("Starting client with URL: {}", url_str);

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

            // Start the iris client with the provided URL
            let client = match iris_client::start_client(&url_str) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to start iris client with URL {}: {:?}", url_str, e);
                    return;
                }
            };

            // Loop over incoming data
            for item in client {
                if let Err(e) = writer.write_all(&item) {
                    eprintln!("Write error: {:?}", e);
                    break;
                }
            }

            eprintln!("Client stream ended for URL: {}", url_str);
        });
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_addSocket(
        mut env: JNIEnv,
        _class: JClass,
        ip_address: JString,
    ) {
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Trace)
                .with_tag("RustApp"),
        );

        // Convert JString to Rust String
        let ip_str: String = match env.get_string(&ip_address) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert IP address from Java: {}", e);
                return;
            }
        };

        info!("Adding socket for IP: {}", ip_str);

        // Add WiFi Aware socket using the iris_client function
        match iris_client::add_wifi_aware_socket(&ip_str) {
            Ok(()) => {
                info!("Successfully added WiFi Aware socket for IP: {}", ip_str);
            }
            Err(e) => {
                log::error!("Failed to add WiFi Aware socket for IP {}: {}", ip_str, e);
            }
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_removeSocket(
        mut env: JNIEnv,
        _class: JClass,
        ip_address: JString,
    ) {
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Trace)
                .with_tag("RustApp"),
        );

        // Convert JString to Rust String
        let ip_str: String = match env.get_string(&ip_address) {
            Ok(s) => s.into(),
            Err(e) => {
                log::error!("Failed to convert IP address from Java: {}", e);
                return;
            }
        };

        info!("Removing socket for IP: {}", ip_str);

        // Remove WiFi Aware socket using the iris_client function
        match iris_client::remove_wifi_aware_socket(&ip_str) {
            Ok(()) => {
                info!(
                    "Successfully requested removal of WiFi Aware socket for IP: {}",
                    ip_str
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to remove WiFi Aware socket for IP {}: {}",
                    ip_str,
                    e
                );
            }
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
        0.5 + variation // Base 0.5% + variation
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
        25.0 + variation // Base 25 Mbps + variation
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
        (now / 10) as i64 // Increases by 1 MB every 10 seconds
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
        ((now % 5) + 1) as i32 // 1 to 5 connections
    }

    #[no_mangle]
    pub extern "system" fn Java_pt_iris_services_RustService_cleanupAllSockets(
        _env: JNIEnv,
        _class: JClass,
    ) {
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Trace)
                .with_tag("RustApp"),
        );

        info!("Cleaning up all WiFi Aware sockets");

        // TODO: Implement actual cleanup when iris_client supports it
        // For now, we'll log the request for cleanup
        log::info!("Cleanup all sockets requested - this will be implemented when iris_client supports bulk cleanup");

        // Placeholder implementation - this should be replaced when iris_client
        // provides a cleanup_all_wifi_aware_sockets function
    }
}
