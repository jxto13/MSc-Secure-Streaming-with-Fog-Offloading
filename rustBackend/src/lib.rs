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
    use my_network_library::server;
    use crate::service::{launch, watch, rperf_start};
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
        
        spawn(|| {
            let _ = grpcserver::start_grpc_server();
        });
        
        // spawn(|| {
        //     rperf_start();
        // });

        // spawn(|| {
        //     let _ = rperf::start_server();
        // });

        // spawn(|| {
        //     let server_addr = ":::8080";
        //     let mut _server = server::start_server();

        // });

    }
}
