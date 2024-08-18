use std::error::Error;
use std::process::{Command, Stdio};
use std::io::{self, Write, BufRead, BufReader};
use std::thread;
use log::{debug, info, error};

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp_millis()
        .init();


    // Attempt to start the receiver
    let receiver = match iris::edge_client::core::start() {
        Ok(receiver) => receiver,
        Err(e) => {
            error!("Failed to start receiver: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to start receiver").into());
        }
    };

    // let args: Vec<_> = "- -fflags nobuffer -flags low_delay".split_whitespace().collect();
    // let args: Vec<_> = " - --no-cache --untimed  --no-demuxer-thread --video-sync=audio --vd-lavc-threads=1".split_whitespace().collect();
    // Start the ffplay (mpv) process
    let mut ffplay = Command::new("mpv")
    // let mut ffplay = Command::new("ffplay")

        .args(&["-", "--cache=yes", "--cache-secs=5", "--speed=1"])
        // .args(&["-"])
        // .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            error!("Failed to start ffplay process: {}", e);
            io::Error::new(io::ErrorKind::Other, "Failed to start ffplay process")
        })?;
    
    // Get handle to ffplay's stdin
    let mut ffplay_stdin = ffplay.stdin.take().ok_or_else(|| {
        error!("Failed to open ffplay stdin");
        io::Error::new(io::ErrorKind::Other, "Failed to open ffplay stdin")
    })?;

    // Spawn a thread to handle data reception
    let receiver_thread = thread::spawn(move || {
        for data in receiver {
            if let Err(e) = ffplay_stdin.write_all(&data) {
                error!("Failed to write to ffplay stdin: {}", e);
                break;
            }
        }
    });

    // Wait for the receiver thread to finish
    receiver_thread.join().map_err(|e| {
        error!("Receiver thread panicked: {:?}", e);
        io::Error::new(io::ErrorKind::Other, "Receiver thread panicked")
    })?;

    // Wait for the ffplay process to finish
    match ffplay.wait() {
        Ok(status) => info!("ffplay exited with: {}", status),
        Err(e) => {
            error!("Failed to wait on ffplay process: {}", e);
            return Err(Box::new(e));
        }
    }

    Ok(())
}


// use gtk::{prelude::*, Entry, Label};
// use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, Orientation, ComboBoxText};
// use std::error::Error;
// use std::process::{Command, Stdio};
// use std::io::{self, Write};
// use std::thread;
// use log::{debug, info, error};
// use std::os::unix::io::{AsRawFd, FromRawFd};

// fn main() -> Result<(), Box<dyn Error>> {
//     // Initialize the logger
//     env_logger::builder()
//         .filter_level(log::LevelFilter::Debug)
//         .format_target(true)
//         .format_timestamp_millis()
//         .init();

//     // Create a new GTK application
//     let app = Application::new(
//         Some("com.example.ffplay"),
//         Default::default(),
//     );

//     app.connect_activate(build_ui);

//     // Run the GTK application
//     app.run();

//     Ok(())
// }

// fn build_ui(app: &gtk::Application) {
//     // Create a new top level window
//     let window = ApplicationWindow::new(app);
//     window.set_title("FFplay GUI");
//     window.set_default_size(800, 0);

//     // Create a vertical box to hold the widgets
//     let vbox = GtkBox::new(Orientation::Vertical, 10);
//     vbox.set_margin_start(20);
//     vbox.set_margin_end(20);
//     window.add(&vbox);

//     // Create a label and combobox for IP and Port selection
//     let ip_port_label = Label::new(Some("Select IP and Port:"));
//     let ip_port_combo = ComboBoxText::new();
//     ip_port_combo.append_text("127.0.0.1:12345");
//     ip_port_combo.append_text("192.168.1.1:54321");
//     ip_port_combo.append_text("10.0.0.1:9999");
//     ip_port_combo.set_active(Some(0));

//     // Create a button to start ffplay
//     let button = Button::with_label("Start FFplay");

//     // Pack widgets into the vertical box
//     vbox.pack_start(&ip_port_label, false, false, 10);
//     vbox.pack_start(&ip_port_combo, false, false, 10);
//     vbox.pack_start(&button, false, false, 20);

//     // Set up the button click event
//     button.connect_clicked(move |_| {
//         let selected_ip_port = ip_port_combo.active_text().unwrap().to_string();
//         let mut parts = selected_ip_port.split(':');
//         let ip = parts.next().unwrap();
//         let port = parts.next().unwrap();
//         start_ffplay(ip, port);
//     });

//     // Show all the widgets
//     window.show_all();
// }

// fn start_ffplay(ip: &str, port: &str) {
//     thread::spawn(move || {
//         // Attempt to start the receiver
//         let receiver = match iris::edge_client::core::start() {
//             Ok(receiver) => receiver,
//             Err(e) => {
//                 error!("Failed to start receiver: {}", e);
//                 return;
//             }
//         };

//         let args: Vec<_> = "- -fflags nobuffer -flags low_delay".split_whitespace().collect();

//         // Start the ffplay process
//         let mut ffplay = match Command::new("ffplay")
//             .args(&args)
//             .stdin(Stdio::piped())
//             .stdout(Stdio::null())
//             .stderr(Stdio::null())
//             .spawn()
//         {
//             Ok(process) => process,
//             Err(e) => {
//                 error!("Failed to start ffplay process: {}", e);
//                 return;
//             }
//         };

//         // Get handle to ffplay's stdin
//         let mut ffplay_stdin = match ffplay.stdin.take() {
//             Some(stdin) => stdin,
//             None => {
//                 error!("Failed to open ffplay stdin");
//                 return;
//             }
//         };

//         // Spawn a thread to handle data reception
//         let receiver_thread = thread::spawn(move || {
//             for data in receiver {
//                 if let Err(e) = ffplay_stdin.write_all(&data) {
//                     error!("Failed to write to ffplay stdin: {}", e);
//                     break;
//                 }
//             }
//         });

//         // Wait for the receiver thread to finish
//         if let Err(e) = receiver_thread.join() {
//             error!("Receiver thread panicked: {:?}", e);
//         }

//         // Wait for the ffplay process to finish
//         match ffplay.wait() {
//             Ok(status) => info!("ffplay exited with: {}", status),
//             Err(e) => error!("Failed to wait on ffplay process: {}", e),
//         }
//     });
// }
