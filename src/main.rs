use sysinfo::{Process, System};

fn main() {
    let system = SystemMonitor::new();
}

//// ====== System Monitor ======
struct SystemMonitor {
    sys: System,
    system_header: SystemHeader,
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
        let mut monitor = Self {
            sys: System::new_all(),
            system_header: Default::default(),
            cpu: Default::default(),
            memory: Default::default(),
            gpu: Default::default(),
            cooling: Default::default(),
            network: Default::default(),
            storage: Default::default(),
            processes: Default::default(),
        };

        monitor.update();
        monitor
    }

    pub fn update(&mut self) {
        // Refresh sysinfo data
        self.sys.refresh_all();
    }
}


/// System Header
#[derive(Default)]
struct SystemHeader {}

impl SystemHeader {}

/// CPU
#[derive(Default)]
struct CPU {}

impl CPU {}

/// Memory
#[derive(Default)]
struct Memory {}

impl Memory {}

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