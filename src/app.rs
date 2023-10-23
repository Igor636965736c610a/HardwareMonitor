use std::collections::btree_map::Values;
use eframe::glow::HasContext;
use egui::{plot::{Line, Legend, Text, PlotPoint, PlotBounds}, Widget, Vec2};
use egui::{containers::*, *};
use sysinfo::{NetworkExt, NetworksExt, ProcessExt, System, SystemExt};
use std::sync::Arc;
use crate::app::mutex::Mutex;
use std::thread;
use core::time::Duration;

//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ProcessManagerApp {
    project: String,
    system:  Arc<Mutex<System>>
}

impl Default for ProcessManagerApp {
    fn default() -> Self {
        Self {
            project: "ProcessManager".to_owned(),
            system: Arc::new(Mutex::new(System::new())),
        }
    }
}

impl ProcessManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let system = Arc::new(Mutex::new(System::new()));
        system.lock().refresh_all();
        let project = "ProcessManager".to_owned();

        Self {
            project,
            system,
        }
    }

    pub fn update_cpu_info(&self)
    {
        let data = Arc::clone(&self.system);

        thread::spawn(move || {
            loop {
                let mut system = data.lock();
                system.refresh_cpu();
                drop(system);
                println!("TEST1SEKUNDA");
                thread::sleep(Duration::from_secs(1));
            }
        });
    }
}

impl eframe::App for ProcessManagerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        let system_lock = self.system.lock();
        let processor = system_lock.global_cpu_info();
    
        println!("CHUJKURWA {}% ", sysinfo::CpuExt::cpu_usage(processor));
    
        let window_size = _frame.info().window_info.size;

        let plot_points: Vec<[f64; 2]> = (0..1000).map(|i| {
            let x = i as f64 * 0.01;
            [x, x.sin()]
        }).collect();

        let plot_points2: Vec<[f64; 2]> = (0..1000).map(|i| {
            let x = i as f64 * 0.01;
            [x, x.sin()]
        }).collect();

        egui::SidePanel::left("left_panel1").show(ctx, |ui|{
            let plot = egui::plot::Plot::new("plot1")
                .height(0.30 * window_size.y)
                .width(0.30 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .reset();

            ui.set_max_width(0.30 * window_size.x);
            ui.set_max_height(0.30 * window_size.y);

            let plot_bounds = PlotBounds::from_min_max([0.0, 0.0], [20.0, 100.0]);

            plot.show(ui, |plot_ui|{
                plot_ui.line(Line::new(plot_points));
                plot_ui.set_plot_bounds(plot_bounds);
            });

            ui.separator();

            egui::Grid::new("grid1")
            .num_columns(6)
            .show(ui, |ui| {
                ui.label("Kolumna 1");
                ui.label("Kolumna 2");
                ui.label("Kolumna 3");
                ui.label("Kolumna 4");
                ui.label("Kolumna 5");
                ui.label("Kolumna 6");
            });
        
        });

        egui::SidePanel::left("left_panel2").show(ctx, |ui|{
            let plot = egui::plot::Plot::new("plot2")
                .height(0.30 * window_size.y)
                .width(0.30 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .reset();
    
            ui.set_max_width(0.30 * window_size.x);
            ui.set_max_height(0.30 * window_size.y);

            plot.show(ui, |plot_ui|{
                plot_ui.line(Line::new(plot_points2));
            });

            egui::Grid::new("grid1").striped(true)
            .num_columns(6)
            .show(ui, |ui| {
                ui.label("Kolumna 1");
                ui.label("Kolumna 2");
                ui.label("Kolumna 3");
                ui.label("Kolumna 4");
                ui.label("Kolumna 5");
                ui.label("Kolumna 6");
            });

            Frame::canvas(ui.style()).show(ui, |ui| {
                ui.ctx().request_repaint();
                let time = ui.input(|i| i.time);
    
                let desired_size = ui.available_width() * vec2(1.0, 0.35);
                let (_id, rect) = ui.allocate_space(desired_size);
    
                let to_screen =
                    emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);
    
                let mut shapes = vec![];
    
                for &mode in &[2, 3, 5] {
                    let mode = mode as f64;
                    let n = 120;
                    let speed = 1.5;
    
                    let points: Vec<Pos2> = (0..=n)
                        .map(|i| {
                            let t = i as f64 / (n as f64);
                            let amp = (time * speed * mode).sin() / mode;
                            let y = amp * (t * std::f64::consts::TAU / 2.0 * mode).sin();
                            to_screen * pos2(t as f32, y as f32)
                        })
                        .collect();
    
                    let thickness = 10.0 / mode as f32;
                    shapes.push(epaint::Shape::line(points, Stroke::new(thickness, Color32::from_additive_luminance(190))));
                }
    
                ui.painter().extend(shapes);
            })
        });
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}

pub struct HardwareInfo
{

}



impl HardwareInfo {
    pub fn get_cpu_info() {
        let mut system = System::new();

        system.refresh_all();

        system.refresh_cpu();

        // let processor = system.global_cpu_info();
    
        // print!("CPU {}% ", sysinfo::CpuExt::cpu_usage(processor));

        
        //let process = system.cpus().first();

        //print!("CPU {}% ", sysinfo::CpuExt::cpu_usage(process.unwrap()));

        //SoundManager::get_volume();

        //println!("NB CPUs: {}", system.cpus().len());
        for (pid, process) in system.processes() {
            println!("[{}] {} {:?} {} {}", pid, process.name(), process.disk_usage(), process.cpu_usage(), process.status());
        }
//
        system.refresh_cpu();

        // system.refresh_cpu(); // Refreshing CPU information.
        // for cpu in system.cpus() {
        //     println!("{}% ", sysinfo::CpuExt::cpu_usage(cpu));
        // }
        // Sleeping to let time for the system to run for long
        // enough to have useful information.
    }
}

struct SoundManager;

impl SoundManager {
    fn get_volume() {
        println!("dupa");
    }
}