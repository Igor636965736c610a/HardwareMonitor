use std::collections::VecDeque;
use egui::{SidePanel, RichText, Color32, Layout, Align, plot, Grid};
use egui::plot::{Line, Legend, PlotBounds, Plot};
use sysinfo::{NetworkExt, NetworksExt,  System, SystemExt, CpuExt, MacAddr};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use core::time::Duration;
use egui::plot::Corner;

//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ProcessManagerApp {
    system_informations: SystemInformations,
    cpus_columns: usize,
    process_manager_mutex_data: Arc<Mutex<ProcessManagerAppMutexData>>,
}

pub struct ProcessManagerAppMutexData{
    system: System,
    total_cpu_usage: i16,
    memory_usage: i16,
    swap_usage: i16,
    network_informations: Vec<NetworkInformations>,
    cpu_performance_data_points: Data<f32>,
    cpus_performance_data_points: Vec<CpuData>,
    memory_usage_data_points: Data<f32>,
    swap_usage_data_points: Data<f32>,
    network_y_plot_bound: f64
}

impl ProcessManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut system = System::new_all();
        for component in system.components() {
            println!("xx {:?}", component);
        }
        system.refresh_all();
        let cpu_brand = system.global_cpu_info().brand().to_string();
        let host_name = system.host_name();
        let os_version = system.os_version();
        let kernel_version = system.kernel_version();
        let network_informations = system.networks().iter()
            .enumerate()
            .map(|(i, x)|{
                NetworkInformations {
                    number: i + 1,
                    interface_name: x.0.to_string(),
                    mac_address: x.1.mac_address(),
                    is_display_on_plot: true,
                    recived_plot_points: None,
                    transmitted_plot_points: None,
                    recived_bytes: 0,
                    transmitted_bytes: 0,
                    total_errors_on_recived: 0,
                    total_errors_on_transmitted: 0,
                }
            }).collect();

        let cpu_performance_data_points = Data::new(20);
        let memory_usage_data_points = Data::new(20);
        let swap_usage_data_points = Data::new(20);
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
            cpus_performance_data_points,
            network_informations,
            network_y_plot_bound: 100.0
        }));

        Self {
            process_manager_mutex_data,
            cpus_columns: 4,
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
                    let process_manager_mutex_data = &mut *arc_process_manager_mutex_data.lock().unwrap();

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

                    // for cpu in process_manager_mutex_data.system.cpus() {
                    //     println!("{}%", cpu.cpu_usage());
                    // }

                    println!("=> networks:");
                    for (interface_name, data) in process_manager_mutex_data.system.networks() {
                        println!("{}: {}/{} B", data.mac_address(), data.received(), data.transmitted());
                    }

                    // println!("=> disks:");
                    // for disk in process_manager_mutex_data.system.disks() {
                    //     println!("name {}", disk.name().to_string_lossy().to_string());
                    //     println!("available_space {}", disk.available_space());
                    //     let file_system = match std::str::from_utf8(disk.file_system()) {
                    //         Ok(value) => {
                    //             value
                    //         }
                    //         Err(_) => {
                    //             panic!("nie panikuj!")
                    //         }
                    //     };
                    //     println!("file_system {}", file_system);
                    //     println!("is_removable {}", disk.is_removable());
                    //     println!("mount_point {}", disk.mount_point().to_string_lossy().to_string());
                    //     println!("total_space {}", disk.total_space());
                    //     println!("kind {:?}", disk.kind());
                    //     println!("");
                    // }

                    // println!("");
                    // println!("ORIGINAL");
                    // println!("=> disks:");
                    // for disk in process_manager_mutex_data.system.disks() {
                    //     println!("{:?}", disk);
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
                    process_manager_mutex_data.network_informations.iter_mut().for_each(|x| {
                        let net_data = process_manager_mutex_data.system.networks().iter()
                            .find(|y| y.1.mac_address().eq(&x.mac_address));
                        match net_data {
                            Some(data) => {
                                if x.is_display_on_plot {
                                    x.recived_bytes = data.1.received();
                                    x.transmitted_bytes = data.1.transmitted();
                                    match &mut x.recived_plot_points {
                                        Some(points) => {
                                            points.push(data.1.received());
                                        }
                                        None => {
                                            let mut plot_points: Data<u64> = Data::new(20);
                                            plot_points.push(data.1.received());
                                            x.recived_plot_points = Some(plot_points);
                                        }
                                    }
                                    match &mut x.transmitted_plot_points {
                                        Some(points) => {
                                            points.push(data.1.transmitted());
                                        }
                                        None => {
                                            let mut plot_points: Data<u64> = Data::new(20);
                                            plot_points.push(data.1.transmitted());
                                            x.transmitted_plot_points = Some(plot_points);
                                        }
                                    }
                                }
                                else {
                                    x.recived_plot_points = None;
                                    x.transmitted_plot_points = None;
                                }
                            }
                            None => {
                                panic!("nie panikuj!");
                            }
                        }    
                    });

                    let mut net_y_bound: u64 = 100;
    
                    for info in &process_manager_mutex_data.network_informations {
                        if let Some(data) = &info.recived_plot_points {
                            if info.is_display_on_plot {
                                for value in &data.data_records {
                                    if *value > net_y_bound {
                                        net_y_bound = *value;
                                    }
                                }
                            }
                        }
                    }
                    for info in &process_manager_mutex_data.network_informations {
                        if let Some(data) = &info.transmitted_plot_points {
                            if info.is_display_on_plot {
                                for value in &data.data_records {
                                    if *value > net_y_bound {
                                        net_y_bound = *value;
                                    }
                                }
                            }
                        }
                    }
                    
                    process_manager_mutex_data.network_y_plot_bound = net_y_bound as f64;              
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
        let mut mutex_data = mutex_data_arc.lock().unwrap();

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

        // let transmitted_network_points: Vec<[f64; 2]> = {
        //     mutex_data.transmitted_network_data_points.data_iter().enumerate().map(|(index, &i)| {
        //         [index as f64, f64::from(i)]
        //     }).collect()
        // };

        // let recived_network_points: Vec<[f64; 2]> = {
        //     mutex_data.recived_network_data_points.data_iter().enumerate().map(|(index, &i)| {
        //         [index as f64, f64::from(i)]
        //     }).collect()
        // };
        
        let plot_bounds = PlotBounds::from_min_max([0.0, 0.0], [20.0, 100.0]);
        let mut max_y_network_plot_bound = mutex_data.network_y_plot_bound;
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
                .legend(Legend::default().position(Corner::LeftTop).background_alpha(0.0))
                .reset();


            let cpu_line = Line::name(Line::new(cpu_points), "cpu %");

            plot.show(ui, |plot_ui|{
                plot_ui.line(cpu_line);

                mutex_data.cpus_performance_data_points.iter().for_each(|x|{
                    match &x.plot_points {
                        Some(points) => {
                            let inner_points: Vec<[f64; 2]> = points.data_iter().enumerate().map(|(index, &i)| {
                                [index as f64, f64::from(i)]
                            }).collect();
                            plot_ui.line(Line::new(inner_points).name(&x.name));
                        }
                        None => {         
                        }
                    };
                });

                plot_ui.set_plot_bounds(plot_bounds);
            });
        
            ui.horizontal(|inner_ui|{
                inner_ui.vertical(|inner_ui|{
                    inner_ui.horizontal(|inner_ui|{
                        inner_ui.label(RichText::new("CPU :"));
                        inner_ui.colored_label(Color32::RED, RichText::new(format!("{}%", mutex_data.total_cpu_usage)));
                    });
                    inner_ui.menu_button("plot", |inner_ui|{
                        mutex_data.cpus_performance_data_points.iter_mut().for_each(|cpu|{
                            inner_ui.checkbox(&mut cpu.is_display_on_plot, &cpu.name);
                        });
                    });
                    inner_ui.add_space(3.0);
                    inner_ui.add(egui::DragValue::new(&mut self.cpus_columns).speed(0.03).clamp_range(1.0..=4.0).suffix(" columns"));
                    inner_ui.set_min_width(63.0)
                });
                inner_ui.separator();
                inner_ui.vertical(|inner_ui|{
                    inner_ui.add_space(3.5);
                    mutex_data.cpus_performance_data_points.chunks(self.cpus_columns).for_each(|x|{
                        inner_ui.horizontal(|inner_ui|{
                            for ele in x {
                                inner_ui.vertical(|inner_ui|{
                                    inner_ui.set_min_width(70.0);
                                    inner_ui.label(format!("{}: {}%",ele.name, ele.usage.round() as i32));
                                    inner_ui.add_space(0.7);
                                });
                            }
                        });
                    })
                })
            });

            ui.separator();

            ui.label(RichText::new(&self.system_informations.cpu_brand).heading());
            match &self.system_informations.os_version {
                Some(value) => ui.label(RichText::new(format!("os version: {}", value))),
                _ => { return; }
            };
            match &self.system_informations.kernel_version {
                Some(value) => ui.label(RichText::new(format!("kernel version: {}", value))),
                _ => { return; }
            };

            ui.separator();

            ui.with_layout(Layout::top_down(Align::Min), |ui|{
                let network_plot: Plot = Plot::new("NETWORK")
                    .show_axes([false, true])
                    .height(0.32 * window_size.y)
                    .width(0.32 * window_size.x)
                    .allow_scroll(false)
                    .allow_drag(false)
                    .legend(Legend::default().position(Corner::RightTop).background_alpha(0.0))
                    .reset();

                // let network_transmitted_line = Line::name(Line::new(transmitted_network_points), "bytes transmitted");
                // let network_recived_line = Line::name(Line::new(recived_network_points), "bytes recived");


                network_plot.show(ui, |plot_ui: &mut plot::PlotUi|{
                    mutex_data.network_informations.iter().for_each(|x| {
                        match &x.transmitted_plot_points {
                            Some(points) => {
                                let transmitted_line: Vec<[f64; 2]> = {
                                    points.data_iter().enumerate().map(|(index, &i)| {
                                        [index as f64, i as f64]
                                    }).collect()
                                };
                                let name = format!("{}. bytes transmitted", x.number);
                                plot_ui.line(Line::new(transmitted_line).name(name));
                            }
                            None => {
                            }                         
                        }
                        match &x.recived_plot_points {
                            Some(points) => {
                                let recived_line: Vec<[f64; 2]> = {
                                    points.data_iter().enumerate().map(|(index, &i)| {
                                        [index as f64, i as f64]
                                    }).collect()
                                };
                                let name = format!("{}. bytes recived", x.number);
                                plot_ui.line(Line::new(recived_line).name(name));
                            }
                            None => {
                            } 
                        }
                    });
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max([0.0, 0.0], [20.0, max_y_network_plot_bound]));
                })
            });

            ui.menu_button("networks", |inner_ui|{
                mutex_data.network_informations.iter_mut().for_each(|net|{
                    let text = format!("{}. {}{}{}", net.number, net.interface_name, "\n", net.mac_address);
                    inner_ui.checkbox(&mut net.is_display_on_plot, text);
                });
            });
        });

        SidePanel::left("MEMORY").show(ctx, |ui|{
            let plot = Plot::new("plot2")
                .show_axes([false, true])
                .height(0.32 * window_size.y)
                .width(0.32 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .legend(Legend::default().position(Corner::LeftTop).background_alpha(0.0))
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

struct DiskInformations{
    name: String,
    available_space: u64,
    file_system: String,
    is_removable: String,
    mount_point: String,
    total_space: u64,
    kind: String,
}

struct NetworkInformations {
    number: usize,
    interface_name: String,
    mac_address: MacAddr,
    is_display_on_plot: bool,
    recived_plot_points: Option<Data<u64>>,
    transmitted_plot_points: Option<Data<u64>>,
    recived_bytes: u64,
    transmitted_bytes: u64,
    total_errors_on_recived: u64,
    total_errors_on_transmitted: u64,
}