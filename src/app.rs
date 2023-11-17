use std::collections::VecDeque;
use egui::epaint::Hsva;
use egui::{SidePanel, RichText, Color32, Layout, Align, plot};
use egui::plot::{Line, Legend, PlotBounds, Plot, Corner, PlotPoints};
use sysinfo::{NetworkExt, NetworksExt,  System, SystemExt, CpuExt, MacAddr, Cpu, DiskExt, RefreshKind, DiskKind};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use core::time::Duration;

//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ProcessManagerApp {
    cpu_informations: CpuInformations,
    system_informations: SystemInformations,
    memory_informations: MemoryInformations,
    cpus_columns: usize,
    process_manager_mutex_data: Arc<Mutex<ProcessManagerAppMutexData>>,
}

pub struct ProcessManagerAppMutexData{
    total_cpu_usage: u64,
    memory_usage: u64,
    swap_usage: u64,
    network_informations: Vec<NetworkInformations>,
    cpu_performance_data_points: Data<f32>,
    cpus_performance_data_points: Vec<CpuData>,
    memory_usage_data_points: Data<f32>,
    disks_informations: Vec<DiskInformations>,
    swap_usage_data_points: Data<f32>,
    network_y_plot_bound: f64
}

impl ProcessManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, sys: &mut System) -> Self {
        sys.refresh_all();
        let cpu_brand = sys.global_cpu_info().brand().to_string();
        let host_name = sys.host_name();
        let os_version = sys.os_version();
        let kernel_version = sys.kernel_version();
        let system_version_full_name = match sys.long_os_version() {
            Some(long_os_version) => {
                long_os_version
            }
            None => {
                match sys.name() {
                    Some(name) => {
                        format!("{} {}", name, sys.distribution_id())
                    }
                    None => {
                        sys.distribution_id()
                    }
                }
            }
        };
        let system_informations = SystemInformations{
            host_name,
            system_version_full_name,
        };
        let disks_informations = sys.disks().iter().map(|x|{
            let name = if format!("{:?}", x.name()).replace("\"", "") != "" {
                format!("{:?}", x.name()).replace("\"", "")
            } else {
                String::from("None")
            };
            let kind = match x.kind() {
                DiskKind::SSD => String::from("SSD"),
                DiskKind::HDD => String::from("HDD"),
                DiskKind::Unknown(_) => String::from("-")
            };
            DiskInformations{
                name,
                available_space: x.available_space(),
                file_system: match std::str::from_utf8(x.file_system()) {
                    Ok(value) => {
                        value.to_string()
                    }
                    Err(_) => {
                        panic!("nie panikuj!")
                    }
                },
                is_removable: if x.is_removable() { String::from("yes") } else { String::from("no") },
                mount_point: format!("{:?}", x.mount_point()).replace("\"", ""),
                total_space: x.total_space(),
                kind: format!("{}", kind)
            }
        }).collect();
        let memory_informations = MemoryInformations{
            total_memory: sys.total_memory(),
            total_swap: sys.total_swap(),
        };
        let network_informations = sys.networks().iter()
            .enumerate()
            .map(|(i, x)|{
                NetworkInformations {
                    number: i + 1,
                    interface_name: x.0.to_string(),
                    mac_address: x.1.mac_address(),
                    is_display_on_plot: true,
                    network_display: None,
                    total_errors_on_recived: 0,
                    total_errors_on_transmitted: 0,
                }
            }).collect();

        let cpu_performance_data_points = Data::new(20);
        let memory_usage_data_points = Data::new(20);
        let swap_usage_data_points = Data::new(20);
        let cpus_performance_data_points: Vec<CpuData> = CpuData::new(sys.cpus(), 1);

        let process_manager_mutex_data = Arc::new(Mutex::new(ProcessManagerAppMutexData{
            total_cpu_usage: 0,
            memory_usage: 0,
            swap_usage: 0,
            cpu_performance_data_points,
            memory_usage_data_points,
            swap_usage_data_points,
            cpus_performance_data_points,
            network_informations,
            disks_informations,
            network_y_plot_bound: 100.0
        }));

        Self {
            process_manager_mutex_data,
            system_informations,
            memory_informations,
            cpus_columns: 4,
            cpu_informations: CpuInformations { 
                cpu_brand,
                kernel_version,
                os_version,
            }
        }
    }

    pub fn start_updating_system_info(&self, mut sys: System)
    {
        let arc_process_manager_mutex_data = Arc::clone(&self.process_manager_mutex_data);

        thread::spawn(move || {
            loop {
                {
                    let process_manager_mutex_data = &mut *arc_process_manager_mutex_data.lock().unwrap();

                    sys.refresh_specifics(RefreshKind::everything()
                        .without_components()
                        .without_components_list()
                        .without_users_list());

                    // for component in process_manager_mutex_data.system.components() {
                    //     println!("xx {:?}", component);
                    // }

                    let processor = sys.global_cpu_info().cpu_usage();
                    let memory = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;
                    let swap =  (sys.used_swap() as f64 / sys.total_swap() as f64) * 100.0;

                    // println!("xxx - {}", process_manager_mutex_data.system.load_average().one);

                    // process_manager_mutex_data.system.refresh_all();

                    // println!("{} {:?} {:?}", sys.distribution_id(), sys.long_os_version(), sys.name());
                    

                    // for (pid, process) in process_manager_mutex_data.system.processes() {
                    //     println!("[{}] {} {:?}", pid, process.name(), process.disk_usage());
                    // }

                    // println!("{}",process_manager_mutex_data.system.global_cpu_info().frequency());

                    // for component in process_manager_mutex_data.system.components() {
                    //     println!("xx {:?}", component);
                    // }

                    // println!("=> disks:");
                    // for disk in sys.disks() {
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

                    // println!("=> networks:");
                    // for (interface_name, data) in process_manager_mutex_data.system.networks() {
                    //     println!("{}: {}/{} B", data.mac_address(), data.received(), data.transmitted());
                    // }

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

                    sys.cpus().iter().for_each(|x|{
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

                    process_manager_mutex_data.total_cpu_usage = processor.round() as u64;
                    process_manager_mutex_data.memory_usage = sys.used_memory();
                    process_manager_mutex_data.swap_usage = sys.used_swap();
                    process_manager_mutex_data.cpu_performance_data_points.push(processor);
                    process_manager_mutex_data.memory_usage_data_points.push(memory as f32);
                    process_manager_mutex_data.swap_usage_data_points.push(swap as f32);
                    process_manager_mutex_data.network_informations.iter_mut().for_each(|x: &mut NetworkInformations| {
                        let net_data = sys.networks().iter()
                            .find(|y| y.1.mac_address().eq(&x.mac_address));
                        match net_data {
                            Some(data) => {
                                x.total_errors_on_recived = data.1.total_errors_on_received();
                                x.total_errors_on_transmitted = data.1.total_errors_on_transmitted();
                                if x.is_display_on_plot {
                                let recived = data.1.received();
                                let transmitted = data.1.transmitted();
                                    match &mut x.network_display {
                                        Some(value) => {
                                            value.recived_plot_points.push(recived);
                                            value.transmitted_plot_points.push(transmitted);
                                        }
                                        None => {
                                            let mut recived_points = Data::new(20);
                                            let mut transmitted_points = Data::new(20);
                                            recived_points.push(recived);
                                            transmitted_points.push(transmitted);
                                            let network_display = NetworkDisplay {
                                                recived_plot_points: recived_points,
                                                transmitted_plot_points: transmitted_points
                                            };

                                            x.network_display = Some(network_display);
                                        }
                                    }
                                }
                                else {
                                    x.network_display = None;
                                }
                            }
                            None => {
                                panic!("nie panikuj!");
                            }
                        }    
                    });

                    let mut net_y_bound: u64 = 100;
    
                    for info in &process_manager_mutex_data.network_informations {
                        if let Some(data) = &info.network_display {
                            if info.is_display_on_plot {
                                for value in data.recived_plot_points.data_iter() {
                                    if *value > net_y_bound {
                                        net_y_bound = *value;
                                    }
                                }
                                for value in data.transmitted_plot_points.data_iter() {
                                    if *value > net_y_bound {
                                        net_y_bound = *value;
                                    }
                                }
                            }
                        }
                    }
                    
                    process_manager_mutex_data.network_y_plot_bound = net_y_bound as f64;              
                }
                thread::sleep(Duration::from_secs(1));
            }
        });
        println!("test")
    }

    pub fn bytes_to_gigabytes(bytes: u64) -> f64 {
        bytes as f64 / 1_073_741_824.0
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
                .legend(Legend::default().position(Corner::LeftTop).background_alpha(0.0));

            let cpu_line = Line::name(Line::new(cpu_points), "cpu %");

            plot.show(ui, |plot_ui|{
                plot_ui.line(cpu_line);

                mutex_data.cpus_performance_data_points.iter().for_each(|x|{
                    match &x.plot_points {
                        Some(points) => {
                            let inner_points: Vec<[f64; 2]> = points.data_iter().enumerate().map(|(index, &i)| {
                                [index as f64, f64::from(i)]
                            }).collect();
                            plot_ui.line(Line::new(inner_points).name(&x.name).color(x.color));
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

            ui.label(RichText::new(&self.cpu_informations.cpu_brand).heading());
            match &self.cpu_informations.os_version {
                Some(value) => ui.label(RichText::new(format!("os version: {}", value))),
                _ => { return; }
            };
            match &self.cpu_informations.kernel_version {
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

                network_plot.show(ui, |plot_ui: &mut plot::PlotUi|{
                    mutex_data.network_informations.iter().for_each(|x| {
                        match &x.network_display {
                            Some(points) => {
                                
                                let transmitted_line: Vec<[f64; 2]> = {
                                    points.transmitted_plot_points.data_iter().enumerate().map(|(index, &i)| {
                                        [index as f64, i as f64]
                                    }).collect()
                                };
                                let recived_line: Vec<[f64; 2]> = {
                                    points.recived_plot_points.data_iter().enumerate().map(|(index, &i)| {
                                        [index as f64, i as f64]
                                    }).collect()
                                };
                                plot_ui.line(Line::new(transmitted_line).name(format!("{}. bytes transmitted", x.number)));
                                plot_ui.line(Line::new(recived_line).name(format!("{}. bytes recived", x.number)));
                            }
                            None => {
                            }                         
                        }
                    });
                    plot_ui.set_plot_bounds(PlotBounds::from_min_max([0.0, 0.0], [20.0, max_y_network_plot_bound]));
                })
            });

            ui.vertical_centered(|inner_ui| {
                mutex_data.network_informations.iter_mut().for_each(|net|{
                    inner_ui.horizontal(|inner_ui| {
                        inner_ui.checkbox(&mut net.is_display_on_plot, "");
                        inner_ui.vertical(|inner_ui| {
                            let mut t1 = RichText::new(format!("{}. {}", net.number, net.interface_name));
                            let mut t2 = RichText::new(format!("Mac address: {}", net.mac_address));
                            if !net.is_display_on_plot {
                                t1 = t1.weak();
                                t2 = t2.weak();
                            }
                            inner_ui.label(t1);
                            inner_ui.label(t2);
                            inner_ui.separator();
                        })
                    });
                });
            });
        });

        SidePanel::left("MEMORY").show(ctx, |ui|{
            let plot = Plot::new("memory_plot")
                .show_axes([false, true])
                .height(0.32 * window_size.y)
                .width(0.32 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .legend(Legend::default().position(Corner::LeftTop).background_alpha(0.0))
                .reset();
    
            ui.set_max_width(0.32 * window_size.x);
            ui.set_max_height(0.32 * window_size.y);

            plot.show(ui, |plot_ui|{
                plot_ui.line(Line::new(memory_points).name("memory %"));
                plot_ui.line(Line::new(swap_points).name("swap %"));
                plot_ui.set_plot_bounds(plot_bounds);
            });

            ui.add_space(2.0);

            let mut memory_section_width: f32 = 0.0;

            ui.vertical(|inner_ui|{
                inner_ui.horizontal(|inner_ui|{
                    let memory_group = inner_ui.group(|inner_ui|{
                        inner_ui.label(RichText::new("Memory used:"));
                        inner_ui.label(RichText::new(format!("{:.2} GB", ProcessManagerApp::bytes_to_gigabytes(mutex_data.memory_usage))).color(Color32::RED));
                        inner_ui.label("/");
                        inner_ui.label(format!("{:.2} GB", ProcessManagerApp::bytes_to_gigabytes(self.memory_informations.total_memory)));
                    });
                    
                    let swap_group = inner_ui.group(|inner_ui|{
                        inner_ui.label(RichText::new("Swap used:"));
                        inner_ui.label(RichText::new(format!("{:.2} GB", ProcessManagerApp::bytes_to_gigabytes(mutex_data.swap_usage))).color(Color32::LIGHT_BLUE));
                        inner_ui.label("/");
                        inner_ui.label(format!("{:.2} GB", ProcessManagerApp::bytes_to_gigabytes(self.memory_informations.total_swap)));
                    });
                    memory_section_width = memory_group.response.rect.width() + swap_group.response.rect.width();
                })
            });

            ui.add_space(1.0);

            ui.separator();

            ui.vertical(|inner_ui|{
                match &self.system_informations.host_name {
                    Some(value) => {
                        inner_ui.horizontal(|inner_ui|{
                            inner_ui.label(RichText::new("Host name:"));
                            inner_ui.label(format!("{}", value));
                        });
                    }
                    None => {
                    }
                }
                inner_ui.horizontal(|inner_ui|{
                    inner_ui.label(RichText::new("System:"));
                    inner_ui.label(format!("{}", self.system_informations.system_version_full_name));
                });
            });

            ui.separator();

            ui.label(RichText::new("Disks:").heading());

            mutex_data.disks_informations.iter().enumerate().for_each(|(i, disk)|{
                ui.group(|inner_ui|{
                    inner_ui.set_width(memory_section_width);

                    inner_ui.horizontal(|inner_ui|{
                        inner_ui.vertical(|inner_ui|{
                            inner_ui.label(RichText::new(format!("{}", disk.name)).size(14.0).underline());
                            inner_ui.label(format!("{}", disk.kind));
                            inner_ui.label(format!("{}", disk.available_space));
                            inner_ui.label(format!("{}", disk.is_removable));
                            inner_ui.label(format!("{}", disk.file_system));
                            inner_ui.label(format!("{}", disk.mount_point));
                            inner_ui.label(format!("{}", disk.total_space));
                        });
                        Plot::new(i)
                            .show_axes([false, true])
                            .allow_scroll(false)
                            .allow_drag(false)
                            .show(inner_ui, |plot_ui|{
                                plot_ui.line(Line::new(PlotPoints::default()));
                                plot_ui.set_plot_bounds(plot_bounds);
                                
                            });
                    });
                });
            });
            // Grid::new("grid1").striped(true)
            // .num_columns(6)
            // .show(ui, |ui| {
            //     ui.label("Kolumna 1");
            //     ui.label("Kolumna 2");
            //     ui.label("Kolumna 3");
            //     ui.label("Kolumna 4");
            //     ui.label("Kolumna 5");
            //     ui.label("Kolumna 6");
            // });
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

struct MemoryInformations {
    total_memory: u64,
    total_swap: u64,
}

struct SystemInformations {
    host_name: Option<String>,
    system_version_full_name: String,
}

struct CpuInformations{
    cpu_brand: String,
    kernel_version: Option<String>,
    os_version: Option<String>,
}

struct CpuData{
    name: String,
    usage: f32,
    color: Color32,
    is_display_on_plot: bool,
    plot_points: Option<Data<f32>>
}

impl CpuData {
    fn new(cpus: &[Cpu], initial_auto_color_index: usize) -> Vec<Self> {
        let mut i = initial_auto_color_index;
        let data = cpus.iter().map(|x|{
            let cpu = Self { name: x.name().to_string(), usage: x.cpu_usage(), is_display_on_plot: false, plot_points: None, color: Self::auto_color(i)};
            i += 2;
            cpu
        }).collect();

        data
    }

    fn auto_color(i: usize) -> Color32 {
        let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0;
        let h = i as f32 * golden_ratio;
        Hsva::new(h, 0.85, 0.5, 1.0).into()
    }
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
    network_display: Option<NetworkDisplay>,
    total_errors_on_recived: u64,
    total_errors_on_transmitted: u64,
}

struct NetworkDisplay {
    recived_plot_points: Data<u64>,
    transmitted_plot_points: Data<u64>,
}