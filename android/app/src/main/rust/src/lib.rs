use log::info;

#[allow(non_snake_case)]
pub mod android {
    use std::thread;

    use android_logger::{self, init_once};
    use jni::objects::{JClass, JString};
    use jni::JNIEnv;
    use log::info;

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
        
        // Start the Rust processing thread and receive the receiver
        if let Ok(receiver) = edge_client::core::start() {

            thread::spawn(||{
                for item in receiver {
                    // info!("{}", item.len());
                }
            });
        } else {
            eprintln!("Failed to start Rust processing thread");
        }


    }
}
