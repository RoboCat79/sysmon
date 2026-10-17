use terminal_size::{terminal_size, Width, Height};
use sysinfo::{Components, System};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use std::thread;
use std::io::{self, Write};

mod ui;
use ui::{
    render_bar, render_box, render_sparkline, print_columns, format_percent, format_bytes,
    BoxLine, BoxStyle, ColorMode, Alignment, resolve_temp_color, strip_ansi_len,
    RESET, RED, GREEN, YELLOW, BLUE, MAGENTA, CYAN, DARK_GRAY,
    clear_and_home, cursor_home, hide_cursor, show_cursor
};

const REFRESH_RATE: Duration = Duration::from_millis(500);
const REFRESH_SPARKLINE: Duration = Duration::from_millis(1000);

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

    pub fn display(&mut self) -> Vec<String> {
        let width = get_terminal_width(80);
        let mut lines = Vec::new();

        lines.extend(self.header.display(width));
        lines.extend(self.cpu.display(width / 2));

        lines
    }
}


/// System Header
#[derive(Debug)]
struct SystemHeader {
    host_name: String,
    os_name: String,
    kernel_version: String,
    architecture: String,
    rust_version: &'static str,

    uptime_seconds: u64,
    load_avg_one: f64,
    load_avg_five: f64,
    load_avg_fifteen: f64,
}

impl SystemHeader {
    pub fn new() -> Self {
        let load = System::load_average();

        let raw_os = System::name().unwrap_or_default();
        let os_name = if raw_os.is_empty() {
            "Linux".to_string()
        } else if !raw_os.to_lowercase().contains("linux") {
            format!("Linux ({})", raw_os)
        } else {
            raw_os
        };

        Self {
            host_name: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            os_name,
            kernel_version: System::kernel_version().unwrap_or_else(|| "".to_string()),
            architecture: System::cpu_arch(),
            rust_version: env!("RUSTC_VERSION"),
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

    /// Produces the content lines to be framed inside the box
    pub fn render_lines(&self) -> Vec<BoxLine> {
        vec![
            BoxLine::Split(
                format!("{CYAN}OS:{RESET} {} {} ({})",
                        self.os_name,
                        self.kernel_version,
                        self.architecture,
                ),
                format!("{CYAN}Uptime:{RESET} {}  {BLUE}|{RESET}  {CYAN}Load Avg:{RESET} {:.2}, {:.2}, {:.2}",
                        self.format_uptime(),
                        self.load_avg_one,
                        self.load_avg_five,
                        self.load_avg_fifteen
                ),
            ),
            BoxLine::Left(
                format!("{CYAN}Rust:{RESET} {}", self.rust_version)
            ),
        ]
    }

    pub fn display(&self, width: usize) -> Vec<String> {
        let lines = self.render_lines();
        let title = format!("{MAGENTA}{} ({}){RESET}", self.host_name, self.kernel_version);

        let style = BoxStyle {
            color_code: BLUE,
            title: Box::leak(title.into_boxed_str()).to_string(), // Or update BoxStyle::title to accept &str / String
            width,
        };

        render_box(&lines, &style)
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

    pub usage_history: VecDeque<f64>,
    pub max_history_len: usize,
    history_interval: Duration,
    last_history_sample: Instant,
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

        let max_history_len = 128;

        let mut cpu = Self {
            name,
            physical_cores,
            logical_cores,
            global_usage: 0.0,
            frequency_mhz: 0,
            temperature_c: 0.0,
            core_usages: vec![0.0; logical_cores],
            usage_history: VecDeque::with_capacity(max_history_len),
            max_history_len,
            history_interval: REFRESH_SPARKLINE,
            last_history_sample: Instant::now() - Duration::from_millis(500),
        };

        cpu.update(sys, components);
        cpu
    }

    pub fn update(&mut self, sys: &System, components: &Components) {
        self.global_usage = sys.global_cpu_usage();
        self.core_usages = sys.cpus().iter().map(|c| c.cpu_usage()).collect();

        if !sys.cpus().is_empty() {
            let total_freq: u64 = sys.cpus().iter().map(|c| c.frequency()).sum();
            self.frequency_mhz = total_freq / sys.cpus().len() as u64;
        }

        self.temperature_c = components
            .iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("cpu") || label.contains("tctl") || label.contains("k10temp") || label.contains("coretemp")
            })
            .and_then(|c| c.temperature())
            .unwrap_or(0.0);

        let now = Instant::now();
        if now.duration_since(self.last_history_sample) >= self.history_interval {
            self.usage_history.push_back(self.global_usage as f64);
            if self.usage_history.len() > self.max_history_len {
                self.usage_history.pop_front();
            }
            self.last_history_sample = now;
        }
    }

    pub fn render_lines(&self, inner_width: usize) -> Vec<BoxLine> {
        let mut lines = Vec::new();

        let content_width = inner_width.saturating_sub(2);

        let global_usage_f64 = self.global_usage as f64;
        let global_pct_str = format_percent(global_usage_f64, 5, 1);
        let prefix = format!("{YELLOW}CPU Usage:{RESET} {} ", global_pct_str);
        let prefix_len = strip_ansi_len(&prefix);

        let dynamic_bar_width = inner_width.saturating_sub(prefix_len + 1).max(5);

        let cpu_bar = render_bar(
            global_usage_f64,
            dynamic_bar_width - 1,
            ColorMode::DynamicThreshold,
            '░',
        );
        lines.push(BoxLine::Left(format!("{}{}", prefix, cpu_bar)));

        let trend_label = format!("{YELLOW}CPU Trend:{RESET} ");
        let trend_label_len = strip_ansi_len(&trend_label);
        let trend_padding = prefix_len.saturating_sub(trend_label_len);

        let trend = render_sparkline(
            &self.usage_history,
            dynamic_bar_width,
            ColorMode::DynamicThreshold,
            '-',
        );
        lines.push(BoxLine::Left(format!("{}{}{}", trend_label, " ".repeat(trend_padding), trend)));

        lines.push(BoxLine::Left(String::new()));

        let temp_color = resolve_temp_color(self.temperature_c);
        lines.push(BoxLine::Left(format!(
            "Temp: {}{:.0}°C{RESET}  {GREEN}|{RESET}  Cores: {} Phys / {} Log  {GREEN}|{RESET}  Freq: {} MHz",
            temp_color,
            self.temperature_c,
            self.physical_cores,
            self.logical_cores,
            self.frequency_mhz
        )));

        lines.push(BoxLine::Left(String::new()));

        let num_cols = 3;
        let col_gap = 2;
        let total_gaps = (num_cols - 1) * col_gap;

        let col_width = content_width.saturating_sub(total_gaps) / num_cols;

        let total_cores = self.core_usages.len();
        let rows_per_col = (total_cores + num_cols - 1) / num_cols;

        for row in 0..rows_per_col {
            let mut row_segments = Vec::new();

            for col in 0..num_cols {
                let idx = row + (col * rows_per_col);
                if idx < total_cores {
                    let core_pct = self.core_usages[idx] as f64;
                    let core_pct_str = format_percent(core_pct, 5, 1);
                    let core_prefix = format!("C{:02}: {} ", idx + 1, core_pct_str);

                    let prefix_len = strip_ansi_len(&core_prefix);
                    let mini_bar_width = col_width.saturating_sub(prefix_len).max(2);

                    let mini_bar = render_bar(
                        core_pct,
                        mini_bar_width,
                        ColorMode::DynamicThreshold,
                        '░',
                    );

                    row_segments.push(format!("{}{}", core_prefix, mini_bar));
                }
            }

            lines.push(BoxLine::Left(row_segments.join("  ")));
        }

        lines
    }

    pub fn display(&self, width: usize) -> Vec<String> {
        let inner_width = width.saturating_sub(2);
        let lines = self.render_lines(inner_width);

        let style = BoxStyle {
            color_code: GREEN,
            title: self.name.clone(),
            width,
        };

        render_box(&lines, &style)
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
/// Returns the current terminal width in character columns,
/// or falls back to `default_width` if the terminal cannot be queried.
pub fn get_terminal_width(default_width: usize) -> usize {
    if let Some((Width(w), Height(_))) = terminal_size() {
        w as usize
    } else {
        default_width
    }
}

/// Returns both (width, height) in columns and rows
pub fn get_terminal_dimensions() -> (usize, usize) {
    if let Some((Width(w), Height(h))) = terminal_size() {
        (w as usize, h as usize)
    } else {
        (100, 30) // Default fallback
    }
}