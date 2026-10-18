use std::collections::VecDeque;
use std::time::{Duration, Instant};
use sysinfo::{System};

use crate::MAX_HISTORY_LEN;
use crate::REFRESH_SPARKLINE;
use crate::ui::{
    render_bar, render_box, render_sparkline, format_percent, format_bytes,
    BoxLine, BoxStyle, ColorMode, strip_ansi_len,
    RESET, RED, BOLD, BOLD_YELLOW,
};

const RAM_COLOR: &str = RED;

#[derive(Debug)]
pub struct Memory {
    pub total_ram: u64,
    pub total_swap: u64,
    pub available_ram: u64,
    pub ram_percent: f64,

    pub used_ram: u64,
    pub used_swap: u64,
    pub free_swap: u64,
    pub swap_percent: f64,

    pub ram_history: VecDeque<f64>,
    pub swap_history: VecDeque<f64>,
    pub max_history_len: usize,
    pub history_interval: Duration,
    pub last_history_sample: Instant,
}

impl Memory {
    // Read stats from the system and create a new Memory instance
    pub fn new(sys: &System) -> Self {
        let max_history_len = MAX_HISTORY_LEN;

        Self {
            total_ram: sys.total_memory(),
            used_ram: sys.used_memory(),
            available_ram: sys.available_memory(),
            ram_percent: 0.0,

            total_swap: sys.total_swap(),
            used_swap: sys.used_swap(),
            free_swap: sys.free_swap(),
            swap_percent: 0.0,

            ram_history: VecDeque::with_capacity(max_history_len),
            swap_history: VecDeque::with_capacity(max_history_len),
            max_history_len,
            history_interval: REFRESH_SPARKLINE,
            last_history_sample: Instant::now() - Duration::from_millis(500),
        }
    }

    pub fn update(&mut self, sys: &System) {
        self.total_ram = sys.total_memory();
        self.used_ram = sys.used_memory();
        self.available_ram = sys.available_memory();

        self.ram_percent = if self.total_ram > 0 {
            (self.used_ram as f64 / self.total_ram as f64) * 100.0
        } else {
            0.0
        };

        self.total_swap = sys.total_swap();
        self.used_swap = sys.used_swap();
        self.free_swap = sys.free_swap();

        self.swap_percent = if self.total_swap > 0 {
            (self.used_swap as f64 / self.total_swap as f64) * 100.0
        } else {
            0.0
        };

        self.record_history_sample();
    }

    pub fn display(&self, width: usize) -> Vec<String> {
        let inner_width = width.saturating_sub(2);
        let lines = self.render_lines(inner_width);

        let title = format!(
            "{BOLD}{RAM_COLOR}{} System RAM ({} Swap){RESET}",
            format_bytes(self.total_ram),
            format_bytes(self.total_swap)
        );

        let style = BoxStyle {
            color_code: RAM_COLOR,
            title,
            width,
        };

        render_box(&lines, &style)
    }

    fn render_lines(&self, inner_width: usize) -> Vec<BoxLine> {
        let content_width = inner_width.saturating_sub(2);
        let mut lines = Vec::new();

        let ram_pct_str = format_percent(self.ram_percent, 5, 1);
        let ram_prefix = format!("{BOLD_YELLOW}RAM Usage:{RESET}  {} ", ram_pct_str);
        let ram_prefix_len = strip_ansi_len(&ram_prefix);
        let dynamic_bar_width = content_width.saturating_sub(ram_prefix_len).max(5);

        let ram_bar = render_bar(
            self.ram_percent,
            dynamic_bar_width.saturating_sub(1),
            ColorMode::DynamicThreshold,
            '░',
        );
        lines.push(BoxLine::Left(format!("{}{}", ram_prefix, ram_bar)));

        let ram_trend_label = format!("{BOLD_YELLOW}RAM Trend:{RESET}  ");
        let ram_trend_len = strip_ansi_len(&ram_trend_label);
        let ram_trend_pad = ram_prefix_len.saturating_sub(ram_trend_len);

        let ram_sparkline = render_sparkline(
            &self.ram_history,
            dynamic_bar_width,
            ColorMode::DynamicThreshold,
            '-',
        );
        lines.push(BoxLine::Left(format!(
            "{}{}{}",
            ram_trend_label,
            " ".repeat(ram_trend_pad),
            ram_sparkline
        )));

        lines.push(BoxLine::Left(format!(
            "Used: {}  {RAM_COLOR}|{RESET}  Avail: {}  {RAM_COLOR}|{RESET}  Total: {}",
            format_bytes(self.used_ram),
            format_bytes(self.available_ram),
            format_bytes(self.total_ram),
        )));

        lines.push(BoxLine::Left(String::new()));

        let swap_pct_str = format_percent(self.swap_percent, 5, 1);
        let swap_prefix = format!("{BOLD_YELLOW}Swap Usage:{RESET} {} ", swap_pct_str);
        let swap_prefix_len = strip_ansi_len(&swap_prefix);
        let swap_bar_width = content_width.saturating_sub(swap_prefix_len).max(5);

        let swap_bar = render_bar(
            self.swap_percent,
            swap_bar_width.saturating_sub(1),
            ColorMode::DynamicThreshold,
            '░',
        );
        lines.push(BoxLine::Left(format!("{}{}", swap_prefix, swap_bar)));

        let swap_trend_label = format!("{BOLD_YELLOW}Swap Trend:{RESET} ");
        let swap_trend_len = strip_ansi_len(&swap_trend_label);
        let swap_trend_pad = swap_prefix_len.saturating_sub(swap_trend_len);

        let swap_sparkline = render_sparkline(
            &self.swap_history,
            swap_bar_width,
            ColorMode::DynamicThreshold,
            '-',
        );
        lines.push(BoxLine::Left(format!(
            "{}{}{}",
            swap_trend_label,
            " ".repeat(swap_trend_pad),
            swap_sparkline
        )));

        lines.push(BoxLine::Left(format!(
            "Used: {}  {RAM_COLOR}|{RESET}  Free: {}  {RAM_COLOR}|{RESET}  Total: {}",
            format_bytes(self.used_swap),
            format_bytes(self.free_swap),
            format_bytes(self.total_swap),
        )));

        lines
    }

    fn record_history_sample(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_history_sample) >= self.history_interval {
            // Push RAM point
            self.ram_history.push_back(self.ram_percent);
            if self.ram_history.len() > self.max_history_len {
                self.ram_history.pop_front();
            }

            // Push Swap point
            self.swap_history.push_back(self.swap_percent);
            if self.swap_history.len() > self.max_history_len {
                self.swap_history.pop_front();
            }

            self.last_history_sample = now;
        }
    }
}