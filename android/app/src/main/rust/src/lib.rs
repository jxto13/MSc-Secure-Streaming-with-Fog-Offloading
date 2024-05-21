#[allow(non_snake_case)]
pub mod android {
    use android_logger::{self, init_once};
    use jni::objects::{JByteArray, JClass, JObject, JString};
    use jni::{objects::JByteBuffer, JNIEnv};
    use log::{error, info};

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
        
        edge_client::test();

    }
}
