use std::collections::{VecDeque, HashMap};
use std::ffi::CString;
use std::time::Instant;
use egui::epaint::Hsva;
use egui::scroll_area::ScrollBarVisibility;
use egui::{SidePanel, RichText, Color32, Layout, Align, plot, ScrollArea, Grid, Label, Sense, Ui, Rect, Pos2};
use egui::plot::{Line, Legend, PlotBounds, Plot, Corner};
use itertools::Itertools;
use sysinfo::{NetworkExt, NetworksExt, System, SystemExt, CpuExt, MacAddr, Cpu, DiskExt, RefreshKind, DiskKind, Pid, ProcessExt, Process, DiskUsage};
use winapi::um::ioapiset::DeviceIoControl;
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use core::time::Duration;
use std::str;
use std::ptr;
use winapi::um::handleapi::CloseHandle;
use winapi::um::fileapi::{CreateFileA, OPEN_EXISTING};
use winapi::um::winioctl::{IOCTL_DISK_PERFORMANCE, DISK_PERFORMANCE};
use winapi::shared::minwindef::{DWORD, FALSE};
use winapi::shared::ntdef::HANDLE;
use rand::prelude::*;


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
    network_y_plot_bound: f64,
    process_informations: HashMap<Pid, ProcessInformations>,
    clicked_process: Option<Pid>,
    processes_sort_option: ProcessesSortOption,
    system: System
}

impl ProcessManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut sys = System::new_all(); 
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
                DiskKind::Unknown(_) => String::from("UNKNOWN")
            };
            let useed_space_tuple = ProcessManagerApp::bytes_to_gb_or_tb_tuple(x.total_space() - x.available_space());
            let total_space_tuple = ProcessManagerApp::bytes_to_gb_or_tb_tuple(x.total_space());
            let file_system = match std::str::from_utf8(x.file_system()) {
                Ok(value) => {
                    value.to_string()
                }
                Err(_) => {
                    panic!("nie panikuj!")
                }
            };
            let is_removable = if x.is_removable() { String::from("yes") } else { String::from("no") };
            let disk_fields = vec![
                name,
                format!("{:?}", x.mount_point()).replace("\"", ""),
                format!("{:.2} {}", useed_space_tuple.0, useed_space_tuple.1),
                format!("{:.2} {}", total_space_tuple.0, total_space_tuple.1),
                format!("{}", kind),
                file_system,
                is_removable,
            ];
            let longest_length = disk_fields.iter().map(|s| s.len()).max().unwrap_or(0);
            let adjusted_disk_fields: Vec<String> = disk_fields
                .iter()
                .map(|s| {
                    let padding = " ".repeat(longest_length.saturating_sub(s.len()));
                    format!("{}{}", s, padding)
                }).collect();

            DiskInformations{
                name: adjusted_disk_fields[0].to_string(),
                mount_point: adjusted_disk_fields[1].to_string(),
                used_space: adjusted_disk_fields[2].to_string(),
                total_space: adjusted_disk_fields[3].to_string(),
                kind: adjusted_disk_fields[4].to_string(),
                file_system: adjusted_disk_fields[5].to_string(),
                is_removable: adjusted_disk_fields[6].to_string(),
                last_performance: None,
                plot_points: Data::new(20),
                y_max_bound: 450.0,
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
            network_y_plot_bound: 100.0,
            process_informations: HashMap::new(),
            clicked_process: None,
            processes_sort_option: ProcessesSortOption::Memory,
            system: sys,
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

    pub fn start_updating_system_info(&self)
    {
        let arc_process_manager_mutex_data = Arc::clone(&self.process_manager_mutex_data);

        let mut last_measurement_time = Instant::now();

        thread::spawn(move || {
            loop {
                {
                    let process_manager_mutex_data = &mut *arc_process_manager_mutex_data.lock().unwrap();
                    let sys = &mut process_manager_mutex_data.system;

                    sys.refresh_specifics(RefreshKind::everything()
                        .without_components()
                        .without_components_list()
                        .without_users_list());

                    let processor = sys.global_cpu_info().cpu_usage();
                    let memory = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;
                    let swap =  (sys.used_swap() as f64 / sys.total_swap() as f64) * 100.0;

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

                    process_manager_mutex_data.process_informations = sys.processes()
                        .iter()
                        .group_by(|x| match x.1.parent() {
                            Some(value) => { value }
                            None => Pid::from(rand::thread_rng().gen_range(10_000..100_000))
                        }).into_iter().map(|x|{
                            let inner_vec: Vec<SecificProcess> = x.1.map(|col|{
                                SecificProcess { 
                                    pid: *col.0, 
                                    name: col.1.name().to_string(), 
                                    cpu: col.1.cpu_usage(), 
                                    memory: col.1.memory() as f32, 
                                    disk: (col.1.disk_usage().read_bytes + col.1.disk_usage().written_bytes) as f32 }
                            }).collect();

                            let sum: f32 = inner_vec.iter().map(|y| y.cpu).sum();
                            let cpu = sum / sys.physical_core_count().unwrap_or(1) as f32;
 
                            (x.0, ProcessInformations {
                                pid: x.0,
                                name: inner_vec.iter().next().unwrap().name.clone(),
                                cpu: cpu,
                                memory: inner_vec.iter().map(|y| y.memory as f32).sum(),
                                disk: inner_vec.iter().map(|y| y.disk).sum(),
                                child_processes: inner_vec,
                            })
                        }).collect();

                    sys.disks().iter().for_each(|x|{
                        let disk = process_manager_mutex_data.disks_informations.iter_mut().find(|disk| 
                            disk.mount_point.trim() == format!("{:?}", x.mount_point()).replace("\"", "").trim().to_string()).unwrap();
                        
                        set_disk_transfer(disk, &mut last_measurement_time);
                        let mut disk_y_max_bound = 450.0;
                        let bound = disk.plot_points.data_iter().max_by(|a, b| a.partial_cmp(b).unwrap()); 
                        match bound {
                            Some(value) => {
                                if *value > disk_y_max_bound{
                                    disk_y_max_bound = *value
                                }
                            }
                            None => {
                            }
                        }
                        disk.y_max_bound = disk_y_max_bound;
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
    }

    fn bytes_to_gb_or_tb_tuple(bytes: u64) -> (f64, String) {
        const GB: u64 = 1_073_741_824; // 1 GB = 2^30 bytes
        const TB: u64 = 1_099_511_627_776; // 1 TB = 2^40 bytes
    
        if bytes >= TB {
            (
                bytes as f64 / TB as f64,
                String::from("TB"),
            )
        } else {
            (
                bytes as f64 / GB as f64,
                String::from("GB"),
            )
        }
    }
}

fn set_disk_transfer(disk: &mut DiskInformations, last_measurement_time: &mut Instant) {
    let disk_name = disk.mount_point.replace("\\", "");
    if let Some(current_performance) = get_disk_performance(&disk_name) {
        if let Some(last) = disk.last_performance {
            unsafe {
                let read_diff = current_performance.BytesRead.QuadPart() - last.BytesRead.QuadPart();
                let write_diff = current_performance.BytesWritten.QuadPart() - last.BytesWritten.QuadPart();
                
                let total_diff = read_diff + write_diff;
                
                let elapsed_time = last_measurement_time.elapsed().as_secs_f64();
                
                // transfer spped/rate (KB/s)
                let transfer_rate_kb_per_sec = (total_diff as f64 / 1024.0) / elapsed_time;
                disk.plot_points.push(transfer_rate_kb_per_sec as f32);
            }
        }
        disk.last_performance = Some(current_performance);

        *last_measurement_time = Instant::now();
    }
}

// FROM C++ https://stackoverflow.com/a/30451751 MY CREATIVE INVERTION AND TRIAL AND ERROR METHOD (It works) and winapi docs and frineds help
fn get_disk_performance(disk_name: &str) -> Option<DISK_PERFORMANCE> {
    unsafe {
        let dev = CreateFileA(
            CString::new(format!("\\\\.\\{}", disk_name)).unwrap().as_ptr(),
            winapi::um::winnt::FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null_mut(),
            OPEN_EXISTING,
            0,
            ptr::null_mut(),
        );

        if HANDLE::is_null(dev) {
            eprintln!("Error while opening disk");
            return None;
        }

        let mut disk_info: DISK_PERFORMANCE = std::mem::zeroed();
        let mut bytes: DWORD = 0;

        //https://learn.microsoft.com/en-us/windows/win32/fileio/disk-management-control-codes
        //https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/System/Ioctl/struct.DISK_PERFORMANCE.html
        if DeviceIoControl(
            dev,
            IOCTL_DISK_PERFORMANCE,
            ptr::null_mut(),
            0,
            &mut disk_info as *mut _ as *mut winapi::ctypes::c_void,
            std::mem::size_of::<DISK_PERFORMANCE>() as DWORD,
            &mut bytes,
            ptr::null_mut(),
        ) == FALSE
        {
            eprintln!("Error in DeviceIoControl");
            CloseHandle(dev);
            return None;
        }

        CloseHandle(dev);

        Some(disk_info)
    }
}

impl eframe::App for ProcessManagerApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        let window_size = _frame.info().window_info.size;

        let mutex_data_arc = Arc::clone(&self.process_manager_mutex_data);
        let mutex_data = &mut *mutex_data_arc.lock().unwrap();

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

        SidePanel::left("left_panel1").resizable(false).show(ctx, |ui|{
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

            ui.label(RichText::new(&self.cpu_informations.cpu_brand).heading().color(Color32::from_rgb(210, 151, 49)));
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

            ScrollArea::vertical().scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden).show(ui, |inner_ui|{
                inner_ui.vertical_centered(|inner_ui| {
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
        });

        SidePanel::left("MEMORY").resizable(false).show(ctx, |ui|{
            let plot = Plot::new("memory_plot")
                .show_axes([false, true])
                .height(0.325 * window_size.y)
                .width(0.325 * window_size.x)
                .allow_scroll(false)
                .allow_drag(false)
                .legend(Legend::default().position(Corner::LeftTop).background_alpha(0.0))
                .reset();
    
            ui.set_max_width(0.325 * window_size.x);
            ui.set_max_height(0.325 * window_size.y);

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
                        let mem_usage = ProcessManagerApp::bytes_to_gb_or_tb_tuple(mutex_data.memory_usage);
                        let mem_total = ProcessManagerApp::bytes_to_gb_or_tb_tuple(self.memory_informations.total_memory);
                        inner_ui.label(RichText::new("Memory used:"));
                        inner_ui.label(RichText::new(format!("{:.2} {}", mem_usage.0, mem_usage.1)).color(Color32::RED));
                        inner_ui.label("/");
                        inner_ui.label(format!("{:.2} {}", mem_total.0, mem_total.1));
                    });
                    
                    let swap_group = inner_ui.group(|inner_ui|{
                        let swap_usage = ProcessManagerApp::bytes_to_gb_or_tb_tuple(mutex_data.swap_usage);
                        let total_swap = ProcessManagerApp::bytes_to_gb_or_tb_tuple(self.memory_informations.total_swap);
                        inner_ui.label(RichText::new("Swap used:"));
                        inner_ui.label(RichText::new(format!("{:.2} {}", swap_usage.0, swap_usage.1)).color(Color32::LIGHT_BLUE));
                        inner_ui.label("/");
                        inner_ui.label(format!("{:.2} {}", total_swap.0, total_swap.1));
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
            
            let background_color = Color32::from_rgb(10, 13, 13); 
            let mut disk_section_width = 0.0;

            ui.horizontal(|inner_ui|{
                let label = inner_ui.label(RichText::new("Disks:").heading());
                let group = inner_ui.group(|inner_ui|{
                    inner_ui.colored_label(Color32::GOLD, "Name");
                    inner_ui.colored_label(Color32::BROWN, "Mount");
                    inner_ui.colored_label(Color32::LIGHT_BLUE, "Used space");
                    inner_ui.colored_label(Color32::LIGHT_GRAY, "Total space");
                    inner_ui.colored_label(Color32::LIGHT_RED, "Kind");
                    inner_ui.colored_label(Color32::LIGHT_GREEN, "Fs");
                    inner_ui.colored_label(Color32::KHAKI, "Removable");
                });
                disk_section_width = label.rect.width() + group.response.rect.width() - 5.0;
            });

            ScrollArea::vertical().scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden).show(ui, |inner_ui|{
                mutex_data.disks_informations.iter().enumerate().for_each(|(i, disk)|{
                    inner_ui.group(|inner_ui|{
                        inner_ui.set_width(disk_section_width);
                    
                        inner_ui.horizontal(|inner_ui|{
                            inner_ui.vertical(|inner_ui|{
                                inner_ui.label(RichText::new(format!("{}", disk.name)).size(12.0).underline().color(Color32::GOLD).background_color(background_color).monospace());
                                inner_ui.add_space(1.7);
                                inner_ui.label(RichText::new(format!("{}", disk.mount_point)).size(12.0).color(Color32::BROWN).background_color(background_color).monospace());
                                inner_ui.label(RichText::new(format!("{}", disk.used_space)).size(12.0).color(Color32::LIGHT_BLUE).background_color(background_color).monospace());
                                inner_ui.label(RichText::new(format!("{}", disk.total_space)).size(12.0).color(Color32::LIGHT_GRAY).background_color(background_color).monospace());
                                inner_ui.label(RichText::new(format!("{}", disk.kind)).size(12.0).color(Color32::LIGHT_RED).background_color(background_color).monospace());
                                inner_ui.label(RichText::new(format!("{}", disk.file_system)).size(12.0).color(Color32::LIGHT_GREEN).background_color(background_color).monospace());
                                inner_ui.label(RichText::new(format!("{}", disk.is_removable)).size(12.0).color(Color32::KHAKI).background_color(background_color).monospace());
                            });
                            Plot::new(i)
                                .show_axes([false, true])
                                .allow_scroll(false)
                                .allow_drag(false)
                                .legend(Legend::default().background_alpha(0.0).position(Corner::RightTop))
                                .show(inner_ui, |plot_ui|{
                                    let transfer_line: Vec<[f64; 2]> = {
                                        disk.plot_points.data_iter().enumerate().map(|(index, &i)| {
                                            [index as f64, i as f64]
                                        }).collect()
                                    };
                                    plot_ui.line(Line::new(transfer_line).name("transfer_rate(KB/s)").color(Color32::GREEN).width(0.4));
                                    plot_ui.set_plot_bounds(PlotBounds::from_min_max([0.0, 0.0], [20.0, (disk.y_max_bound + (disk.y_max_bound * 0.23)) as f64]));
                                });
                        });
                    });
                });
            });
        });

        SidePanel::left("Processes").resizable(false).show(ctx, |ui|{
            ui.set_width(0.355 * window_size.x);

            match mutex_data.clicked_process {
                Some(value) => {
                    Grid::new("ClickedProcess")
                    .num_columns(5)
                    .striped(false)
                    .spacing([17.0, 2.0])
                    .show(ui, |inner_ui| {
                        let color = Color32::LIGHT_BLUE;
                        columns_definition_display(mutex_data, inner_ui, color);
                        inner_ui.add(Label::new(RichText::new(format!("{}", '🗙')).color(color)).sense(Sense::click())).clicked().then(||{
                            mutex_data.clicked_process = None;
                        });
                        
                        inner_ui.end_row();
                        
                        let clicked_process = &mutex_data.process_informations.get(&value);

                        match clicked_process {
                            Some(clicked_process) => {
                                inner_ui.with_layout(Layout::default(), |inner_ui|{
                                    inner_ui.set_min_width(165.0);
                                    inner_ui.set_max_width(165.0);
                                    inner_ui.colored_label(color, format!("{} {}", clicked_process.name, '⏷'))
                                });
                                    
                                inner_ui.colored_label(color,format!("{:.1}", clicked_process.cpu));
                                inner_ui.colored_label(color,format!("{:.1}", clicked_process.memory / 1000.0 / 1000.0));
                                inner_ui.colored_label(color,format!("{:.1}", clicked_process.disk / 1000.0 / 1000.0));
                                inner_ui.add(Label::new(RichText::new(format!("{}", '🗙')).color(Color32::from_rgb(210, 151, 49))).sense(Sense::click())).clicked().then(||{
                                    let process_to_kill = mutex_data.system.process(value);
                                    match process_to_kill {
                                        Some(value) => {
                                            value.kill();
                                        }
                                        None => {
                                        }
                                    }
                                });
        
                                inner_ui.end_row();
        
                                let sorted_processes = match mutex_data.processes_sort_option {
                                    ProcessesSortOption::Memory => {
                                        clicked_process.child_processes.iter()
                                            .sorted_by(|a, b| a.memory.partial_cmp(&b.memory).unwrap().reverse())
                                    }
                                    ProcessesSortOption::Cpu => {
                                        clicked_process.child_processes.iter()
                                            .sorted_by(|a, b| a.cpu.partial_cmp(&b.cpu).unwrap().reverse())
                                    }
                                    ProcessesSortOption::Disk => {
                                        clicked_process.child_processes.iter()
                                            .sorted_by(|a, b| a.disk.partial_cmp(&b.disk).unwrap().reverse())
                                    }
                                };
                                
                                sorted_processes.for_each(|process|{
                                    inner_ui.with_layout(Layout::default(), |inner_ui|{
                                        inner_ui.set_min_width(165.0);
                                        inner_ui.set_max_width(165.0);     
                                        inner_ui.label(format!("{}", process.name));  
                                    });
                                    inner_ui.label(format!("{:.1}", process.cpu));
                                    inner_ui.label(format!("{:.1}", process.memory / 1000.0 / 1000.0));
                                    inner_ui.label(format!("{:.1}", process.disk / 1000.0 / 1000.0));
                                    inner_ui.add(Label::new(RichText::new(format!("{}", '🗙')).color(Color32::from_rgb(210, 151, 49))).sense(Sense::click())).clicked().then(||{
                                        let process_to_kill = mutex_data.system.process(process.pid);
                                        match process_to_kill {
                                            Some(value) => {
                                                value.kill();
                                            }
                                            None => {
                                            }
                                        }
                                    });
                                    inner_ui.end_row();
                                });
                            }
                            None => {

                            }
                        }

                    });
                }
                None => {

                }
            }

            ui.separator();

            ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |inner_ui|{
                Grid::new("grid1")
                    .num_columns(5)
                    .striped(false)
                    .spacing([17.0, 2.0])
                    .show(inner_ui, |inner_ui| {
                        let color = Color32::from_rgb(210, 151, 49);
                        columns_definition_display(mutex_data, inner_ui, color);

                        inner_ui.end_row();

                        let processes = &mutex_data.process_informations;
                        let sorted_option = &mutex_data.processes_sort_option;

                        let sorted_processes = get_sorted_processes(sorted_option, processes);

                        sorted_processes.for_each(|process|{
                            
                            inner_ui.with_layout(Layout::default(), |inner_ui|{
                                inner_ui.set_min_width(165.0);
                                inner_ui.set_max_width(165.0);     
                                inner_ui.add(Label::new(format!("{}", process.1.name)).sense(Sense::click())).clicked().then(||{
                                    match &mutex_data.clicked_process {
                                        Some(value) => {
                                            if *value != *process.0 {
                                                mutex_data.clicked_process = Some(*process.0)
                                            }
                                            else {
                                                mutex_data.clicked_process = None
                                            }
                                        }
                                        None => {
                                            mutex_data.clicked_process = Some(*process.0)
                                        }
                                    }
                                });
                            });
                                
                            inner_ui.label(format!("{:.1}", process.1.cpu));
                            inner_ui.label(format!("{:.1}", process.1.memory / 1000.0 / 1000.0));
                            inner_ui.label(format!("{:.1}", process.1.disk / 1000.0 / 1000.0));

                            inner_ui.add(Label::new(RichText::new(format!("{}", '🗙')).color(color)).sense(Sense::click())).clicked().then(||{
                                let process_to_kill = mutex_data.system.process(*process.0);
                                match process_to_kill {
                                    Some(value) => {
                                        value.kill();
                                    }
                                    None => {
                                    }
                                }
                            });
                            inner_ui.end_row();
                        });
                    });
                });
            });
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}

fn get_sorted_processes<'a>(sorted_option: &'a ProcessesSortOption, processes: &'a HashMap<Pid, ProcessInformations>) -> std::vec::IntoIter<(&'a Pid, &'a ProcessInformations)> {
    let sorted_processes = match sorted_option {
        ProcessesSortOption::Memory => {
            processes.iter()
                .sorted_by(|a, b| a.1.memory.partial_cmp(&b.1.memory).unwrap().reverse())
        }
        ProcessesSortOption::Cpu => {
            processes.iter()
                .sorted_by(|a, b| a.1.cpu.partial_cmp(&b.1.cpu).unwrap().reverse())
        }
        ProcessesSortOption::Disk => {
            processes.iter()
                .sorted_by(|a, b| a.1.disk.partial_cmp(&b.1.disk).unwrap().reverse())
        }
    };
    sorted_processes
}

fn columns_definition_display(mutex_data: &mut ProcessManagerAppMutexData, inner_ui: &mut Ui, color: Color32) {
    let unicode_char = '⏷';
    match mutex_data.processes_sort_option {
        ProcessesSortOption::Memory => {
            inner_ui.add(Label::new(RichText::new("NAME").color(color)));
            inner_ui.add(Label::new(RichText::new("CPU ").color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Cpu);
            inner_ui.add(Label::new(RichText::new(format!("MEMORY {}", unicode_char)).color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Memory);
            inner_ui.add(Label::new(RichText::new("DISK ").color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Disk);
        }
        ProcessesSortOption::Cpu => {
            inner_ui.add(Label::new(RichText::new("NAME").color(color)));
            inner_ui.add(Label::new(RichText::new(format!("CPU {}", unicode_char)).color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Cpu);
            inner_ui.add(Label::new(RichText::new("MEMORY ").color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Memory);
            inner_ui.add(Label::new(RichText::new("DISK ").color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Disk);
        }
        ProcessesSortOption::Disk => {
            inner_ui.add(Label::new(RichText::new("NAME").color(color)));
            inner_ui.add(Label::new(RichText::new("CPU ").color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Cpu);
            inner_ui.add(Label::new( RichText::new("MEMORY ").color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Memory);
            inner_ui.add(Label::new( RichText::new(format!("DISK {}", unicode_char)).color(color))
                .sense(Sense::click()))
                .clicked()
                .then(||mutex_data.processes_sort_option = ProcessesSortOption::Disk);
        }
    }
}

pub enum ProcessesSortOption {
    Cpu,
    Memory,
    Disk
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
    mount_point: String,
    used_space: String,
    total_space: String,
    kind: String,
    file_system: String,
    is_removable: String,
    last_performance: Option<DISK_PERFORMANCE>,
    plot_points: Data<f32>,
    y_max_bound: f32,
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

struct ProcessInformations {
    pid: Pid,
    name: String,
    cpu: f32,
    memory: f32,
    disk: f32,
    child_processes: Vec<SecificProcess>
}

struct SecificProcess {
    pid: Pid,
    name: String,
    cpu: f32,
    memory: f32,
    disk: f32,
}