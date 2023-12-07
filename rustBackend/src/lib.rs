mod grpcserver;

#[allow(non_snake_case)]
pub mod android {
    use std::thread::spawn;

    use android_logger::{self, init_once};
    use jni::{
        objects::{JClass, JString},
        JNIEnv,
    };
    use my_network_library::server;
    use crate::grpcserver;

    #[no_mangle]
    pub extern "system" fn Java_com_example_final_1test_1wifiaware_services_RustService_startService(
        _: JNIEnv,
        _: JClass,
        _: JString,
    ) {
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("RustApp"),
        );
        
        // Log that the Rust app is starting
        log::info!("Rust app is starting...");

        spawn(|| {
            // Log that the gRPC server is starting
            log::info!("Starting gRPC server...");
            let _ = grpcserver::start_grpc_server();
            log::info!("gRPC server started");
        });

        spawn(|| {
            // Log that the network server is starting
            log::info!("Starting network server...");
            let mut _server = server::start_server();
            log::info!("Network server started");
        });

        // Log that the Rust app has finished starting
        log::info!("Rust app started.");

    }
}
