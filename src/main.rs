use std::io::{self, Write};
use std::thread;
use std::time::{Duration};
use sysinfo::{Components, System};
use terminal_size::{Height, Width, terminal_size};

mod ui;
use ui::{
    get_terminal_width, clear_and_home, cursor_home, hide_cursor, show_cursor
};
mod header;
use header::Header;
mod cpu;
use cpu::CPU;
mod memory;
use memory::Memory;
mod gpu;
use gpu::GPU;
mod cooling;
use cooling::Cooling;
mod network;
use network::Network;
mod storage;
use storage::Storage;
mod processes;
use processes::Processes;

const REFRESH_RATE: Duration = Duration::from_millis(500);
const REFRESH_SPARKLINE: Duration = Duration::from_millis(1000);
const MAX_HISTORY_LEN: usize = 128;

fn main() {
    let mut sysmon = SystemMonitor::new();
    hide_cursor();
    clear_and_home();

    thread::sleep(Duration::from_millis(200));
    sysmon.update();

    let mut last_size = (0usize, 0usize);

    loop {
        let current_size = if let Some((Width(w), Height(h))) = terminal_size() {
            (w as usize, h as usize)
        } else {
            (100, 30)
        };

        if current_size != last_size {
            clear_and_home();
            last_size = current_size;
        } else {
            cursor_home();
        }

        sysmon.update();

        for line in sysmon.display() {
            println!("{}\x1b[K", line);
        }

        let _ = io::stdout().flush();

        thread::sleep(REFRESH_RATE);
    }
}

//// ====== System Monitor ======
struct SystemMonitor {
    sys: System,
    components: Components,
    header: Header,
    cpu: CPU,
    memory: Memory,
    // gpu: GPU,
    // cooling: Cooling,
    // network: Network,
    // storage: Storage,
    // processes: Processes,
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
            header: Header::new(),
            cpu: CPU::new(&sys, &components),
            memory: Memory::new(&sys),
            // gpu: Default::default(), 
            // cooling: Default::default(), 
            // network: Default::default(), 
            // storage: Default::default(), 
            // processes: Default::default(),
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

    pub fn display(&mut self) -> Vec<String> {
        let width = get_terminal_width(80);
        let mut lines = Vec::new();

        lines.extend(self.header.display(width));
        lines.extend(self.cpu.display(width / 2));

        lines
    }
}