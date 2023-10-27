use std::collections::{btree_map::Values, VecDeque};
use eframe::glow::HasContext;
use egui::{plot::{Line, Legend, Text, PlotPoint, PlotBounds}, Widget, Vec2};
use egui::{containers::*, *};
use env_logger::fmt::Formatter;
use sysinfo::{NetworkExt, NetworksExt, ProcessExt, System, SystemExt, Component, ComponentExt, CpuExt};
use std::sync::Arc;
use crate::app::mutex::Mutex;
use std::thread;
use core::time::Duration;
use egui::plot::Corner;

//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ProcessManagerApp {
    project: String,
    system:  Arc<Mutex<System>>,
    cpu_performance_data_points: Arc<Mutex<Data<f32>>>,
    memory_usage_data_points: Arc<Mutex<Data<f32>>>,
    swap_usage_data_points: Arc<Mutex<Data<f32>>>
}

impl ProcessManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let system = Arc::new(Mutex::new(System::new_all()));
        system.lock().refresh_all();
        let project = "ProcessManager".to_owned();
        let cpu_performance_data_points = Arc::new(Mutex::new(Data::new(20)));
        let memory_usage_data_points = Arc::new(Mutex::new(Data::new(20)));
        let swap_usage_data_points = Arc::new(Mutex::new(Data::new(20)));


        Self {
            project,
            system,
            cpu_performance_data_points,
            memory_usage_data_points,
            swap_usage_data_points
        }
    }

    pub fn start_updating_system_info(&self)
    {
        let arc_system = Arc::clone(&self.system);
        let arc_cpu_data_points = Arc::clone(&self.cpu_performance_data_points);
        let arc_memory_usage_points = Arc::clone(&self.memory_usage_data_points);
        let arc_swap_usage_points = Arc::clone(&self.swap_usage_data_points);

        thread::spawn(move || {
            loop {
                {
                    let mut system = arc_system.lock();
                    let mut cpu_data_points = arc_cpu_data_points.lock();
                    let mut memory_usage_points = arc_memory_usage_points.lock();
                    let mut swap_usage_points = arc_swap_usage_points.lock();

                    system.refresh_cpu();
                    system.refresh_memory();
                    system.refresh_all();

                    let processor = system.global_cpu_info();
                    let memory = (system.used_memory() as f64 / system.total_memory() as f64) * 100.0;
                    let swap =  (system.used_swap() as f64 / system.total_swap() as f64) * 100.0;

                    for (interface_name, network) in system.networks() {
                        println!("in: {} xxxxxxx {} ----------- {} ------------ {}", network.received(), network.transmitted(), network.mac_address(), interface_name);
                    }

                    cpu_data_points.push(sysinfo::CpuExt::cpu_usage(processor));
                    memory_usage_points.push(memory as f32);
                    swap_usage_points.push(swap as f32);
                }

                //println!("TEST1SEKUNDA");
                thread::sleep(Duration::from_secs(1));
            }
        });
        println!("test")
    }
}

impl eframe::App for ProcessManagerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        let window_size = _frame.info().window_info.size;

        let cpu_points: Vec<[f64; 2]> = {
            let cpu_performance_data_points_lock = self.cpu_performance_data_points.lock();

            cpu_performance_data_points_lock.data_iter().enumerate().map(|(index, &i)| {
                [index as f64, f64::from(i)]
            }).collect()
        };

        let memory_points: Vec<[f64; 2]> = {
            let memory_usage_data_points_lock = self.memory_usage_data_points.lock();
            
            memory_usage_data_points_lock.data_iter().enumerate().map(|(index, &i)| {
                [index as f64, f64::from(i)]
            }).collect()
        };

        let swap_points: Vec<[f64; 2]> = {
            let swap_usage_data_points_lock = self.swap_usage_data_points.lock();

            swap_usage_data_points_lock.data_iter().enumerate().map(|(index, &i)| {
                [index as f64, f64::from(i)]
            }).collect()
        };
        
        let plot_bounds = PlotBounds::from_min_max([0.0, 0.0], [20.0, 100.0]);

        egui::SidePanel::left("left_panel1").show(ctx, |ui|{
            let plot = egui::plot::Plot::new("plot1")
                .show_axes([false, true])
                .height(0.30 * window_size.y)
                .width(0.30 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .legend(Legend::default().position(Corner::LeftTop))
                .reset();

            ui.set_max_width(0.30 * window_size.x);
            ui.set_max_height(0.30 * window_size.y);

            let cpu_line = plot::Line::name(Line::new(cpu_points), "cpu %");

            plot.show(ui, |plot_ui|{
                plot_ui.line(cpu_line);
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
                .show_axes([false, true])
                .height(0.30 * window_size.y)
                .width(0.30 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .legend(Legend::default().position(Corner::LeftTop))
                .reset();
    
            ui.set_max_width(0.30 * window_size.x);
            ui.set_max_height(0.30 * window_size.y);

            let memory_line = plot::Line::name(Line::new(memory_points), "memory %");
            let swap_line = plot::Line::name(Line::new(swap_points), "swap %");

            plot.show(ui, |plot_ui|{
                plot_ui.line(memory_line);
                plot_ui.line(swap_line);
                plot_ui.set_plot_bounds(plot_bounds);
            });

            ui.separator();

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

pub struct Data<T>{
    data_points: usize,
    data_records: VecDeque<T>,
}

impl<T> Data<T> {
    pub fn new(data_points: usize) -> Self {
        Self {
            data_points,
            data_records: VecDeque::with_capacity(data_points),
        }
    }

    pub fn push(&mut self, data_record: T) {
        self.data_records.push_back(data_record);
        if self.data_records.len() > self.data_points + 1 {
            self.data_records.pop_front();
        }
    }

    pub fn data_iter(&self) -> impl Iterator<Item = &T> {
        self.data_records.iter()
    }
}

struct SoundManager;

impl SoundManager {
    fn get_volume() {
        println!("dupa");
    }
}