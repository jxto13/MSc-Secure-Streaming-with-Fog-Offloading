mod service;
mod grpcserver;

#[allow(non_snake_case)]
pub mod android {
    use std::thread::spawn;

    use android_logger::{self, init_once};
    use jni::{
        objects::{JClass, JString},
        JNIEnv,
    };

    use crate::service::{launch, watch};
    use crate::grpcserver;

    #[no_mangle]
    pub extern "system" fn Java_com_example_final_1test_1wifiaware_services_RustService_startService(
        _: JNIEnv,
        _: JClass,
        _: JString,
    ) {
        init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Trace)
                .with_tag("RustApp"),
        );
        
        spawn(|| {
            let _ = grpcserver::start_grpc_server();
        });

        spawn(|| {
            rperf::start_server();
        });
        

    }
}
