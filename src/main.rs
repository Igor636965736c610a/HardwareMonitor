#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
use std::{sync::mpsc, thread::Thread};
use std::thread;
use sysinfo::{NetworkExt, NetworksExt, ProcessExt, System, SystemExt};

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ProcessManager",
        native_options,
        Box::new(|cc| {
            let mut app = ProcessManager::ProcessManagerApp::new(cc);
            
            app.update_cpu_info();
            
            Box::new(app)
        }),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(eframe_template::ProcessManagerApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
