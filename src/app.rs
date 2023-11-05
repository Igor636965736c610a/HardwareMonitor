use std::{collections::{btree_map::Values, VecDeque}, fmt::{Debug, Formatter}, path::Components};
use eframe::glow::HasContext;
use egui::{plot::{Line, Legend, Text, PlotPoint, PlotBounds}, Widget, Vec2, panel::PanelState, epaint::ahash::{HashMap, HashMapExt}};
use egui::{containers::*, *};
use serde::de::value;
use sysinfo::{NetworkExt, NetworksExt, ProcessExt, System, SystemExt, Component, ComponentExt, CpuExt, RefreshKind, Cpu};
use std::sync::Arc;
use crate::app::mutex::Mutex;
use std::thread;
use core::time::Duration;
use egui::plot::Corner;
use crate::app::plot::Plot;

//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ProcessManagerApp {
    project: String,
    system_informations: SystemInformations,
    process_manager_mutex_data: Arc<Mutex<ProcessManagerAppMutexData>>,
}

pub struct ProcessManagerAppMutexData{
    system: System,
    total_cpu_usage: i16,
    memory_usage: i16,
    swap_usage: i16,
    cpu_performance_data_points: Data<f32>,
    cpus_performance_data_points: Vec<CpuData>,
    memory_usage_data_points: Data<f32>,
    swap_usage_data_points: Data<f32>,
    transmitted_network_data_points: Data<f32>,
    recived_network_data_points: Data<f32>,
} 

impl ProcessManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        for component in system.components() {
            println!("xx {:?}", component);
        }
        let cpu_brand = system.global_cpu_info().brand().to_string();
        let host_name = system.host_name();
        let os_version = system.os_version();
        let kernel_version = system.kernel_version();

        let cpu_performance_data_points = Data::new(20);
        let memory_usage_data_points = Data::new(20);
        let swap_usage_data_points = Data::new(20);
        let transmitted_network_data_points = Data::new(20);
        let recived_network_data_points = Data::new(20);
        let cpus_performance_data_points: Vec<CpuData> = system.cpus().iter().map(|x|{
            CpuData { name: x.name().to_string(), usage: x.cpu_usage(), is_display_on_plot: false, plot_points: None }
        }).collect();

        let process_manager_mutex_data = Arc::new(Mutex::new(ProcessManagerAppMutexData{
            system,
            total_cpu_usage: 0,
            memory_usage: 0,
            swap_usage: 0,
            cpu_performance_data_points,
            memory_usage_data_points,
            swap_usage_data_points,
            transmitted_network_data_points,
            recived_network_data_points,
            cpus_performance_data_points,
        }));

        
        let project = "ProcessManager".to_owned();

        Self {
            project,
            process_manager_mutex_data,
            system_informations: SystemInformations { 
                cpu_brand,
                kernel_version,
                os_version,
                host_name,
            }
        }
    }

    pub fn start_updating_system_info(&self)
    {
        let arc_process_manager_mutex_data = Arc::clone(&self.process_manager_mutex_data);

        thread::spawn(move || {
            loop {
                {
                    let process_manager_mutex_data = &mut *arc_process_manager_mutex_data.lock();

                    process_manager_mutex_data.system.refresh_cpu();
                    process_manager_mutex_data.system.refresh_memory();
                    process_manager_mutex_data.system.refresh_networks_list();
                    process_manager_mutex_data.system.refresh_networks();
                    process_manager_mutex_data.system.refresh_disks_list();
                    process_manager_mutex_data.system.refresh_disks();
                    process_manager_mutex_data.system.refresh_components_list();
                    process_manager_mutex_data.system.refresh_components();

                    // for component in process_manager_mutex_data.system.components() {
                    //     println!("xx {:?}", component);
                    // }

                    let processor = process_manager_mutex_data.system.global_cpu_info().cpu_usage();
                    let memory = (process_manager_mutex_data.system.used_memory() as f64 / process_manager_mutex_data.system.total_memory() as f64) * 100.0;
                    let swap =  (process_manager_mutex_data.system.used_swap() as f64 / process_manager_mutex_data.system.total_swap() as f64) * 100.0;

                    // println!("xxx - {}", process_manager_mutex_data.system.load_average().one);

                    // process_manager_mutex_data.system.refresh_all();

                    // for (pid, process) in process_manager_mutex_data.system.processes() {
                    //     println!("[{}] {} {:?}", pid, process.name(), process.disk_usage());
                    // }

                    // println!("{}",process_manager_mutex_data.system.global_cpu_info().frequency());

                    // for component in process_manager_mutex_data.system.components() {
                    //     println!("xx {:?}", component);
                    // }

                    // println!("=> disks:");
                    // for disk in process_manager_mutex_data.system.disks() {
                    //    println!("{:?}", disk);
                    // }

                    // for cpu in system.cpus() {
                    //     println!("{} - {}% ",cpu.name(), cpu.cpu_usage());
                    // }

                    // for (interface_name, network) in system.networks() {
                    //     println!("in: {} xxxxxxx {} ----------- {} ------------ {}", network.received(), network.transmitted(), network.mac_address(), interface_name);
                    // }

                    let network_main_interface = match process_manager_mutex_data.system.networks().iter().next(){
                        Some(network_main_interface) => {
                            //println!("in: {} xxxxxxx {} ----------- {} ------------ {}", 
                            //    network_main_interface.1.received(), network_main_interface.1.transmitted(), network_main_interface.1.mac_address(), network_main_interface.0);

                            network_main_interface
                        }
                        None => {
                            panic!("xd");
                        }
                    };

                    // for cpu in process_manager_mutex_data.system.cpus() {
                    //     println!("{}%", cpu.cpu_usage());
                    // }

                    process_manager_mutex_data.system.cpus().iter().for_each(|x|{
                        let cpu_data = process_manager_mutex_data.cpus_performance_data_points.iter_mut().find(|y| y.name == x.name());
                        match cpu_data {
                            Some(cpu) => {
                                cpu.usage = x.cpu_usage();
                                if cpu.is_display_on_plot {
                                    match &mut cpu.plot_points {
                                        Some(plot_points) => {
                                            plot_points.push(cpu.usage);
                                        }
                                        None => {
                                            let mut plot_points: Data<f32> = Data::new(20);
                                            plot_points.push(cpu.usage);
                                            cpu.plot_points = Some(plot_points);
                                        }
                                    }
                                }
                                else {
                                    cpu.plot_points = None;
                                }
                            }
                            None => {
                                panic!("nie panikuj!");
                            }
                        }
                    });

                    process_manager_mutex_data.total_cpu_usage = processor.round() as i16;
                    process_manager_mutex_data.memory_usage = memory.round() as i16;
                    process_manager_mutex_data.swap_usage = swap.round() as i16;
                    process_manager_mutex_data.cpu_performance_data_points.push(processor);
                    process_manager_mutex_data.memory_usage_data_points.push(memory as f32);
                    process_manager_mutex_data.swap_usage_data_points.push(swap as f32);
                    process_manager_mutex_data.transmitted_network_data_points.push(network_main_interface.1.transmitted() as f32);
                    process_manager_mutex_data.recived_network_data_points.push(network_main_interface.1.received() as f32)
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

        let mutex_data_arc = Arc::clone(&self.process_manager_mutex_data);
        let mut mutex_data = mutex_data_arc.lock();

        let cpu_points: Vec<[f64; 2]> = {
            mutex_data.cpu_performance_data_points.data_iter().enumerate().map(|(index, &i)| {
                [index as f64, f64::from(i)]
            }).collect()
        };

        let memory_points: Vec<[f64; 2]> = {
            mutex_data.memory_usage_data_points.data_iter().enumerate().map(|(index, &i)| {
                [index as f64, f64::from(i)]
            }).collect()
        };

        let swap_points: Vec<[f64; 2]> = {
            mutex_data.swap_usage_data_points.data_iter().enumerate().map(|(index, &i)| {
                [index as f64, f64::from(i)]
            }).collect()
        };

        let transmitted_network_points: Vec<[f64; 2]> = {
            mutex_data.transmitted_network_data_points.data_iter().enumerate().map(|(index, &i)| {
                [index as f64, f64::from(i)]
            }).collect()
        };

        let recived_network_points: Vec<[f64; 2]> = {
            mutex_data.recived_network_data_points.data_iter().enumerate().map(|(index, &i)| {
                [index as f64, f64::from(i)]
            }).collect()
        };
        
        let plot_bounds = PlotBounds::from_min_max([0.0, 0.0], [20.0, 100.0]);
        let mut max_y_network_plot_bound = ProcessManagerApp::find_max_y(&[&recived_network_points, &transmitted_network_points]);
        max_y_network_plot_bound += max_y_network_plot_bound * 0.19;

        SidePanel::left("left_panel1").show(ctx, |ui|{
            ui.set_max_width(0.32 * window_size.x);
            ui.set_max_height(0.32 * window_size.y);

            let plot = Plot::new("CPU")
                .show_axes([false, true])
                .height(0.32 * window_size.y)
                .width(0.32 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .legend(Legend::default().position(Corner::LeftTop))
                .reset();


            let cpu_line = Line::name(Line::new(cpu_points), "cpu %");

            plot.show(ui, |plot_ui|{
                plot_ui.line(cpu_line);
                plot_ui.set_plot_bounds(plot_bounds);
            });
            
            
            ui.horizontal(|inner_ui|{
                inner_ui.vertical(|inner_ui|{
                    inner_ui.horizontal(|inner_ui|{
                        inner_ui.label("CPU :");
                        inner_ui.colored_label(Color32::RED, format!("{}%", mutex_data.total_cpu_usage));
                    });
                    inner_ui.menu_button("plot", |inner_ui|{
                        mutex_data.cpus_performance_data_points.iter_mut().for_each(|cpu|{
                            inner_ui.checkbox(&mut cpu.is_display_on_plot, &cpu.name);
                        });
                    });
                    inner_ui.set_min_width(63.0)
                });
                inner_ui.separator();
                inner_ui.vertical(|inner_ui|{
                    mutex_data.cpus_performance_data_points.chunks(4).for_each(|x|{
                        inner_ui.horizontal(|inner_ui|{
                            for ele in x {
                                inner_ui.vertical(|inner_ui|{
                                    inner_ui.set_min_width(70.0);
                                    inner_ui.label(format!("{}: {}%",ele.name, ele.usage.round() as i32));
                                });
                            }
                        });
                    })
                })
            });

            ui.separator();

            ui.with_layout(Layout::top_down(Align::Min), |ui_test|{
                let network_plot: Plot = Plot::new("NETWORK")
                    .show_axes([false, true])
                    .height(0.32 * window_size.y)
                    .width(0.32 * window_size.x)
                    .allow_scroll(false)
                    .allow_drag(false)
                    .legend(Legend::default().position(Corner::RightTop))
                    .reset();

                let network_transmitted_line = Line::name(Line::new(transmitted_network_points), "bytes transmitted");
                let network_recived_line = Line::name(Line::new(recived_network_points), "bytes recived");


                network_plot.show(ui_test, |plot_ui: &mut plot::PlotUi|{
                    plot_ui.line(network_transmitted_line);
                    plot_ui.line(network_recived_line);
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max([0.0, 0.0], [20.0, max_y_network_plot_bound]));
                })
            });
        });

        SidePanel::left("MEMORY").show(ctx, |ui|{
            let plot = Plot::new("plot2")
                .show_axes([false, true])
                .height(0.32 * window_size.y)
                .width(0.32 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .legend(Legend::default().position(Corner::LeftTop))
                .reset();
    
            ui.set_max_width(0.32 * window_size.x);
            ui.set_max_height(0.32 * window_size.y);

            let memory_line = Line::name(Line::new(memory_points), "memory %");
            let swap_line = Line::name(Line::new(swap_points), "swap %");

            plot.show(ui, |plot_ui|{
                plot_ui.line(memory_line);
                plot_ui.line(swap_line);
                plot_ui.set_plot_bounds(plot_bounds);
            });

            Grid::new("grid1").striped(true)
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
        ctx.request_repaint();
    }
}

impl ProcessManagerApp {
    fn find_max_y(arrays: &[&Vec<[f64; 2]>]) -> f64 {
        let mut max_y = 0.0;
    
        for array in arrays {
            for point in *array {
                if point[1] > max_y {
                    max_y = point[1];
                }
            }
        }
    
        if max_y == 0.0 {
            return 100.0;
        }
    
        max_y
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

struct SystemInformations{
    cpu_brand: String,
    kernel_version: Option<String>,
    os_version: Option<String>,
    // distribution: String,
    host_name: Option<String>,

}

struct CpuData{
    name: String,
    usage: f32,
    is_display_on_plot: bool,
    plot_points: Option<Data<f32>>
}

struct SoundManager;

impl SoundManager {
    fn get_volume() {
        println!("dupa");
    }
}