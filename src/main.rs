#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use sysinfo::{SystemExt};

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "ProcessManager",
        native_options,
        Box::new(|cc| {
            let mut system = sysinfo::System::new_all();

            let app = ProcessManager::ProcessManagerApp::new(cc, &mut system);

            app.start_updating_system_info(system);
            
            Box::new(app)
        }),
    )
}