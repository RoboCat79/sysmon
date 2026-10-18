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
use crate::ui::box_columns;

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
pub struct SystemMonitor {
    pub system: System,
    pub components: Components,
    pub header: Header,
    pub cpu: CPU,
    pub memory: Memory,
    pub gpu: GPU,
    // pub cooling: Cooling,
    // pub network: Network,
    pub storage: Storage,
    // pub processes: Processes,
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
            gpu: GPU::new(),
            // cooling: Default::default(),
            // network: Default::default(),
            storage: Storage::new(),
            // processes: Default::default(),
            system: sys,
            components,
        }
    }

    pub fn update(&mut self) {
        // Refresh sysinfo data
        self.system.refresh_all();
        self.components.refresh(true);

        // Update each component
        self.header.update();
        self.cpu.update(&self.system, &self.components);
        self.memory.update(&self.system);
        self.storage.update();
        self.gpu.update();
    }

    pub fn display(&mut self) -> Vec<String> {
        let term_width = get_terminal_width(80);
        let mut lines = Vec::new();

        lines.extend(self.header.display(term_width));

        let col_gap = 2;
        let left_width = (term_width.saturating_sub(col_gap)) / 2;
        let right_width = term_width.saturating_sub(left_width + col_gap);

        let left_box = self.cpu.display(left_width);
        let mut right_box = self.memory.display(right_width);
        right_box.extend(self.storage.display(right_width));
        lines.extend(box_columns(&left_box, &right_box, left_width));

        lines.extend(self.gpu.display(term_width));

        lines
    }
}