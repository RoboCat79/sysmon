use terminal_size::{terminal_size, Width, Height};
use sysinfo::{Components, System};
use std::thread;
use std::time::Duration;
use std::collections::VecDeque;

fn main() {

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

    pub fn display(&mut self) {
        //self.header.display(get_terminal_width(64));
        //self.cpu.display();
        //self.memory.display();
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
            title: Box::leak(title.into_boxed_str()), // Or update BoxStyle::title to accept &str / String
            width,
        };

        render_box_advanced(&lines, &style)
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

        let max_history_len = 24;

        let mut cpu = Self {
            name,
            physical_cores,
            logical_cores,
            global_usage: 0.0,
            frequency_mhz: 0,
            temperature_c: 0.0,
            core_usages: vec![0.0; logical_cores],
            usage_history: VecDeque::with_capacity(max_history_len),
            max_history_len
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

        if self.usage_history.len() >= self.max_history_len {
            self.usage_history.pop_front();
        }
        self.usage_history.push_back(self.global_usage as f64);
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
pub fn format_bytes(bytes: u64) -> String {
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

//// === Render Helpers ===
const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const WHITE: &str = "\x1b[37m";
const DARK_GRAY: &str = "\x1b[90m";

#[derive(Clone, Copy)]
pub enum ColorMode {
    // Changes color dynamically based on load thresholds
    DynamicThreshold,

    // Fixed static color regardless of percentage
    Fixed(&'static str),

    // Custom 24-bit TrueColor RGB: FixedRgb(r, g, b)
    FixedRgb(u8, u8, u8),
}
fn resolve_color(mode: ColorMode, percent: f64) -> String {
    match mode {
        ColorMode::DynamicThreshold => {
            if percent >= 95.0 {
                RED.to_string()
            } else if percent >= 70.0 {
                YELLOW.to_string()
            } else {
                GREEN.to_string()
            }
        }
        ColorMode::Fixed(ansi) => ansi.to_string(),
        ColorMode::FixedRgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
    }
}

/// Renders a horizontal bar of a fixed character width
/// `percent`: value from 0.0 tp 100.0
/// `width`: numbre of chracters wide the bar should be
pub fn render_bar(percent: f64, width: usize, mode: ColorMode, track_char: char) -> String {
    let clamped = percent.clamp(0.0, 100.0);
    let filled_len = ((clamped / 100.0) * width as f64).round() as usize;
    let empty_len = width - filled_len;

    let color = resolve_color(mode, clamped);

    let filled_str = "█".repeat(filled_len);
    let empty_str: String = std::iter::repeat(track_char).take(empty_len).collect();

    format!("{color}{filled_str}{DARK_GRAY}{empty_str}{RESET}")
}

/// Renders a sparkline / histogram string from a history slice.
/// Takes values between 0.0 and 100.0.
pub fn render_sparkline(history: &VecDeque<f64>, width: usize, mode: ColorMode, pad_char: char) -> String {
    const SPARK_CHARS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let mut result = String::with_capacity(width);

    if history.len() < width {
        let pad_len = width - history.len();
        let pad_str: String = std::iter::repeat(pad_char).take(pad_len).collect();
        result.push_str(&format!("{DARK_GRAY}{pad_str}{RESET}"))
    }

    let start_idx = history.len().saturating_sub(width);
    for &val in history.range(start_idx..) {
        let clamped = val.clamp(0.0, 100.0);

        let index = (((clamped / 100.0) * 7.0).round() as usize).min(7);
        let color = resolve_color(mode, clamped);
        result.push_str(&format!("{color}{}{RESET}", SPARK_CHARS[index]));
    }

    result
}

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

pub struct BoxStyle {
    color_code: &'static str,
    title: &'static str,
    width: usize,
}
pub fn render_box(lines: &[String], style: &BoxStyle) -> Vec<String> {
    let color = style.color_code;
    let width = style.width;

    let mut output = Vec::new();

    let title_segment = if style.title.is_empty() {
        String::new()
    } else {
        format!(" {} ", style.title)
    };

    let title_char_len = strip_ansi_len(&title_segment);
    let border_space = width.saturating_sub(2 + title_char_len);
    let left_dashes = border_space / 2;
    let right_dashes = border_space.saturating_sub(left_dashes);

    let top_border = format!(
        "{color}╭{}{title_segment}{}╮{RESET}",
        "─".repeat(left_dashes),
        format!("{color}{}", "─".repeat(right_dashes))
    );
    output.push(top_border);

    for line in lines {
        let visible_len = strip_ansi_len(line);
        let inner_width = width.saturating_sub(2);
        let padding_needed = inner_width.saturating_sub(visible_len);

        output.push(format!(
            "{color}│{RESET} {}{}{color}│{RESET}",
            line,
            " ".repeat(padding_needed.saturating_sub(1))
        ));
    }

    let bottom_border = format!(
        "{color}╰{}╯{RESET}",
        "─".repeat(width.saturating_sub(2))
    );
    output.push(bottom_border);

    output
}

pub enum BoxLine {
    Left(String),
    Center(String),
    Right(String),
    Split(String, String), // Left text, Right text
}

pub fn render_box_advanced(lines: &[BoxLine], style: &BoxStyle) -> Vec<String> {
    const RESET: &str = "\x1b[0m";
    let color = style.color_code;
    let width = style.width;
    let inner_width = width.saturating_sub(2);

    let mut output = Vec::new();

    let title_segment = if style.title.is_empty() {
        String::new()
    } else {
        format!(" {} ", style.title)
    };
    let title_char_len = strip_ansi_len(&title_segment);
    let border_space = width.saturating_sub(2 + title_char_len);
    let left_dashes = border_space / 2;
    let right_dashes = border_space.saturating_sub(left_dashes);

    output.push(format!(
        "{color}╭{}{}{title_segment}{color}{}╮{RESET}",
        "─".repeat(left_dashes),
        RESET,
        "─".repeat(right_dashes)
    ));

    for line in lines {
        let formatted_content = match line {
            BoxLine::Left(text) => pad_aligned(text, inner_width, Alignment::Left),
            BoxLine::Center(text) => pad_aligned(text, inner_width, Alignment::Center),
            BoxLine::Right(text) => pad_aligned(text, inner_width, Alignment::Right),
            BoxLine::Split(left, right) => justify_split(left, right, inner_width),
        };

        output.push(format!("{color}│{RESET}{}{color}│{RESET}", formatted_content));
    }

    // 3. Build Bottom Border
    output.push(format!(
        "{color}╰{}╯{RESET}",
        "─".repeat(width.saturating_sub(2))
    ));

    output
}

/// Count visible character length ignoring ANSI color escape sequences
fn strip_ansi_len(s: &str) -> usize {
    let mut in_escape = false;
    let mut count = 0;

    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            count += 1;
        }
    }
    count
}

pub fn print_columns(left_box: &[String], right_box: &[String]) {
    let max_lines = left_box.len().max(right_box.len());

    for i in 0..max_lines {
        let left_str = left_box.get(i).map(|s| s.as_str()).unwrap_or("");
        let right_str = right_box.get(i).map(|s| s.as_str()).unwrap_or("");

        println!("{}  {}", left_str, right_str);
    }
}

pub enum Alignment {
    Left,
    Center,
    Right,
}

/// Pads a formatted string inside a target width according to the alignment.
pub fn pad_aligned(text: &str, target_width: usize, align: Alignment) -> String {
    let visible_len = strip_ansi_len(text);
    let total_padding = target_width.saturating_sub(visible_len);

    match align {
        Alignment::Left => {
            // 1-space left margin, remainder on right
            let left_pad = 1.min(total_padding);
            let right_pad = total_padding.saturating_sub(left_pad);
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        Alignment::Center => {
            let left_pad = total_padding / 2;
            let right_pad = total_padding.saturating_sub(left_pad);
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        Alignment::Right => {
            // Remainder on left, 1-space right margin
            let right_pad = 1.min(total_padding);
            let left_pad = total_padding.saturating_sub(right_pad);
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
    }
}

/// Pushes `left` to the left edge and `right` against the right edge.
pub fn justify_split(left: &str, right: &str, inner_width: usize) -> String {
    let left_len = strip_ansi_len(left);
    let right_len = strip_ansi_len(right);

    // Leave a 1-character margin on both left and right edges
    let available_middle = inner_width.saturating_sub(left_len + right_len + 2);

    format!(" {}{}{} ", left, " ".repeat(available_middle), right)
}