use sysinfo::System;
use crate::ui::{render_box, BoxLine, BoxStyle, RESET, BLUE, BOLD, BOLD_CYAN, BOLD_MAGENTA};
const HEADER_COLOR: &str = BLUE;

#[derive(Debug)]
pub struct Header {
    pub host_name: String,
    pub os_name: String,
    pub kernel_version: String,
    pub architecture: String,
    pub rust_version: &'static str,

    pub uptime_seconds: u64,
    pub load_avg_one: f64,
    pub load_avg_five: f64,
    pub load_avg_fifteen: f64,
}

impl Header {
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

    pub fn display(&self, width: usize) -> Vec<String> {
        let lines = self.render_lines();
        let title = format!("{BOLD_MAGENTA}{} ({}){RESET}", self.host_name, self.kernel_version);

        let style = BoxStyle {
            color_code: HEADER_COLOR,
            title: Box::leak(title.into_boxed_str()).to_string(), // Or update BoxStyle::title to accept &str / String
            width,
        };

        render_box(&lines, &style)
    }

    /// Produces the content lines to be framed inside the box
    fn render_lines(&self) -> Vec<BoxLine> {
        vec![
            BoxLine::Split(
                format!("{BOLD_CYAN}OS:{RESET} {} {} ({})",
                        self.os_name,
                        self.kernel_version,
                        self.architecture,
                ),
                format!("{BOLD_CYAN}Uptime:{RESET} {}  {BLUE}|{RESET}  {BOLD_CYAN}Load Avg:{RESET} {:.2}, {:.2}, {:.2}",
                        self.format_uptime(),
                        self.load_avg_one,
                        self.load_avg_five,
                        self.load_avg_fifteen
                ),
            ),
            BoxLine::Left(
                format!("{BOLD_CYAN}Rust:{RESET} {}", self.rust_version)
            ),
        ]
    }

    /// Format Uptime to DD:HH:MM:SS
    fn format_uptime(&self) -> String {
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