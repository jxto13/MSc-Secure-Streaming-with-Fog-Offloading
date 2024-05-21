mod grpcserver;
mod receiver;

#[allow(non_snake_case)]
pub mod android {
    use std::fs::{self, File, OpenOptions};
    use std::io::{BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write};
    use std::os::fd::{FromRawFd, RawFd};
    use std::path::Path;
    use std::slice;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;

    use android_logger::{self, init_once};
    use jni::objects::{JByteArray, JClass, JObject, JString};
    use jni::{objects::JByteBuffer, JNIEnv};
    use log::{error, info};

    use serde_json::error;

    use crate::receiver;

    static mut OPEN_FILE: Option<File> = None;
    static mut DATA_RECEIVER: Option<Receiver<Vec<u8>>> = None;

    #[no_mangle]
    pub extern "system" fn Java_com_example_final_1test_1wifiaware_services_RustService_startService(
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


        let file = match File::open("/data/data/com.example.final_test_wifiaware/files/sample_1280x720_surfing_with_audio.ts") {
            Ok(file) => file,
            Err(e) => {
                error!("Error opening file: {}", e);
                return;
            }
        };

        let (sender, receiver) = mpsc::channel();


        std::thread::spawn(|| {
            receiver::start(sender);

        });


        // Log that the Rust app has finished starting
        info!("Rust app started.");

        // Store the receiver in a global static variable
        unsafe {
            OPEN_FILE = Some(file);
            DATA_RECEIVER = Some(receiver);
        }
    }

    #[no_mangle]
    pub extern "system" fn Java_com_example_final_1test_1wifiaware_services_RustService_helloByte<'a>(
        env: JNIEnv<'a>,
        _class: JClass<'a>,
        size: usize,
    ) -> JByteArray<'a> {
        // Initialize data to an empty vector
        let mut data: Vec<u8> = Vec::new();

        // if let Some(file) = unsafe { &OPEN_FILE } {
        //     let mut reader = BufReader::with_capacity(size, file);
        //     let mut buffer = vec![0; size];
            
        //     match reader.read(&mut buffer) {
        //         Ok(_) => {
        //             // info!("{:?}",buffer);
        //             data = buffer;
        //         }
        //         Err(e) => error!("{}",e),
        
        //     }
        // }
        
        if let Some(receiver) = unsafe {&DATA_RECEIVER} {
            data = receiver.try_recv().unwrap();
        }
        
        // Convert the Rust byte vector to a Java byte array
        let output = env.byte_array_from_slice(&data)
            .expect("Failed to create byte array from slice");

        output
    }


}
