use sysinfo::{Components, System};
use std::thread;
use std::time::Duration;

fn main() {
    let system = SystemMonitor::new();
}

//// ====== System Monitor ======
struct SystemMonitor {
    sys: System,
    components: Components,
    header: SystemHeader,
    cpu: CPU,
    memory: Memory,
    gpu: GPU,
    cooling: Cooling,
    network: Network,
    storage: Storage,
    processes: Processes,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        let mut components = Components::new_with_refreshed_list();

        sys.refresh_cpu_all();
        thread::sleep(Duration::from_millis(200));
        sys.refresh_all();
        components.refresh(true);

        Self {
            header: SystemHeader::new(),
            cpu: CPU::new(&sys, &components),
            memory: Memory::new(&sys),
            gpu: Default::default(),
            cooling: Default::default(),
            network: Default::default(),
            storage: Default::default(),
            processes: Default::default(),
            sys,
            components,
        }
    }

    pub fn update(&mut self) {
        // Refresh sysinfo data
        self.sys.refresh_all();
        self.components.refresh(true);

        // Update each component
        self.header.update();
        self.cpu.update(&self.sys, &self.components);
        self.memory.update(&self.sys);
    }
}


/// System Header
#[derive(Debug)]
struct SystemHeader {
    host_name: String,
    os_name: String,
    kernel_version: String,
    architecture: String,

    uptime_seconds: u64,
    load_avg_one: f64,
    load_avg_five: f64,
    load_avg_fifteen: f64,
}

impl SystemHeader {
    pub fn new() -> Self {
        let load = System::load_average();

        Self {
            host_name: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            os_name: System::name().unwrap_or_else(|| "Linux".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "".to_string()),
            architecture: System::cpu_arch(),
            uptime_seconds: System::uptime(),
            load_avg_one: load.one,
            load_avg_five: load.five,
            load_avg_fifteen: load.fifteen,
        }
    }

    pub fn update(&mut self) {
        let load = System::load_average();
        self.uptime_seconds = System::uptime();
        self.load_avg_one = load.one;
        self.load_avg_five = load.five;
        self.load_avg_fifteen = load.fifteen;
    }

    /// Format Uptime to DD:HH:MM:SS
    pub fn format_uptime(&self) -> String {
        let days = self.uptime_seconds / 86400;
        let hours = (self.uptime_seconds % 86400) / 3600;
        let minutes = (self.uptime_seconds % 3600) / 60;
        let seconds = self.uptime_seconds % 60;

        if days > 0 {
            format!("{:02}:{:02}:{:02}:{:02}", days, hours, minutes, seconds)
        } else {
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        }
    }
}

/// CPU
#[derive(Debug)]
struct CPU {
    name: String,
    physical_cores: usize,
    logical_cores: usize,

    global_usage: f32,
    frequency_mhz: u64,
    temperature_c: f32,
    core_usages: Vec<f32>,
}

impl CPU {
    pub fn new(sys: &System, components: &Components) -> Self {
        let name = sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let physical_cores = sysinfo::System::physical_core_count().unwrap_or(0);
        let logical_cores = sys.cpus().len();

        let mut cpu = Self {
            name,
            physical_cores,
            logical_cores,
            global_usage: 0.0,
            frequency_mhz: 0,
            temperature_c: 0.0,
            core_usages: vec![0.0; logical_cores],
        };

        cpu.update(sys, components);
        cpu
    }

    pub fn update(&mut self, sys: &System, components: &Components) {
        // Overall usage percentage accross all cores
        self.global_usage = sys.global_cpu_usage();

        // Per-core usages
        self.core_usages = sys.cpus().iter().map(|c| c.cpu_usage()).collect();

        // Average frequency across cores (in MHz)
        if !sys.cpus().is_empty() {
            let total_freq: u64 = sys.cpus().iter().map(|c| c.frequency()).sum();
            self.frequency_mhz = total_freq / sys.cpus().len() as u64;
        }

        // Temperature: look for CPU / Tctl / k10temp / coretemp sensor
        self.temperature_c = components
            .iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("cpu") || label.contains("tctl") || label.contains("k10temp") || label.contains("coretemp")
            })
            .and_then(|c| c.temperature())
            .unwrap_or(0.0);
    }
}

/// Memory
#[derive(Debug)]
struct Memory {
    total_ram: u64,
    total_swap: u64,

    used_ram: u64,
    used_swap: u64,
}

impl Memory {
    // Read stats from the system and create a new Memory instance
    fn new(sys: &System) -> Self {
        Self {
            total_ram: sys.total_memory(),
            total_swap: sys.total_swap(),
            used_ram: sys.used_memory(),
            used_swap: sys.used_swap(),
        }
    }

    fn update(&mut self, sys: &System) {
        self.used_ram = sys.used_memory();
        self.used_swap = sys.used_swap();
    }

    // Calculate usage percentages
    fn ram_usage(&self) -> f64 {
        if self.total_ram == 0 {
            return 0.0;
        }
        (self.used_ram as f64 / self.total_ram as f64) * 100.0
    }
    fn swap_usage(&self) -> f64 {
        if self.total_swap == 0 {
            return 0.0;
        }
        (self.used_swap as f64 / self.total_swap as f64) * 100.0
    }
}

/// GPU
#[derive(Default)]
struct GPU {}

impl GPU {}

/// Cooling
#[derive(Default)]
struct Cooling {}

impl Cooling {}

/// Network
#[derive(Default)]
struct Network {}

impl Network {}

/// Storage
#[derive(Default)]
struct Storage {}

impl Storage {}

/// Processes
#[derive(Default)]
struct Processes {}

impl Processes {}

//// ====== HELPER FUNCTIONS ======
/// Format Bytes
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];


    let mut size = bytes as f64;
    let mut unit_index = 0;


    while size > 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }


    if unit_index == 0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index - 1])
    }
}