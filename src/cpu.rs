use std::collections::VecDeque;
use std::time::{Duration, Instant};
use sysinfo::{Components, System};

use crate::MAX_HISTORY_LEN;
use crate::REFRESH_SPARKLINE;
use crate::ui::{
    render_bar, render_box, render_sparkline, format_percent,
    BoxLine, BoxStyle, ColorMode, resolve_temp_color, strip_ansi_len,
    RESET, GREEN, BOLD, BOLD_YELLOW,
};

const CPU_COLOR: &str = GREEN;
const NUM_CORE_COLS: usize = 3;
const NUM_CORE_GAP: usize = 2;

#[derive(Debug)]
pub struct CPU {
    pub name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,

    pub global_usage: f32,
    pub frequency_mhz: u64,
    pub temperature_c: f32,
    pub core_usages: Vec<f32>,

    pub usage_history: VecDeque<f64>,
    pub max_history_len: usize,
    pub history_interval: Duration,
    pub last_history_sample: Instant,
}

impl CPU {
    pub fn new(sys: &System, components: &Components) -> Self {
        let name = sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        let physical_cores = System::physical_core_count().unwrap_or(0);
        let logical_cores = sys.cpus().len();
        let max_history_len = MAX_HISTORY_LEN;

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

        self.temperature_c = Self::get_temperature_c(components);
        self.record_history_sample();
    }

    pub fn display(&self, width: usize) -> Vec<String> {
        let inner_width = width.saturating_sub(2);
        let lines = self.render_lines(inner_width);

        let title = format!(
            "{BOLD}{CPU_COLOR}{}{RESET}",
            self.name,
        );

        let style = BoxStyle {
            color_code: CPU_COLOR,
            title,
            width,
        };

        render_box(&lines, &style)
    }

    fn render_lines(&self, inner_width: usize) -> Vec<BoxLine> {
        let content_width = inner_width.saturating_sub(2);
        let mut lines = Vec::new();

        lines.extend(self.render_summary(content_width));

        lines.push(BoxLine::Left(String::new()));

        lines.extend(self.render_core_grid(content_width, NUM_CORE_COLS, NUM_CORE_GAP));

        lines
    }

    fn render_summary(&self, content_width: usize) -> Vec<BoxLine> {
        let mut lines = Vec::new();

        let global_usage_f64 = self.global_usage as f64;
        let global_pct_str = format_percent(global_usage_f64, 5, 1);
        let prefix = format!("{BOLD_YELLOW}CPU Usage:{RESET} {} ", global_pct_str);
        let prefix_len = strip_ansi_len(&prefix);

        let dynamic_bar_width = content_width.saturating_sub(prefix_len).max(5);

        let cpu_bar = render_bar(
            global_usage_f64,
            dynamic_bar_width - 1,
            ColorMode::DynamicThreshold,
            '░',
        );
        lines.push(BoxLine::Left(format!("{}{}", prefix, cpu_bar)));

        let trend_label = format!("{BOLD_YELLOW}CPU Trend:{RESET} ");
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
            "Temp: {}{:.0}°C{RESET}  {CPU_COLOR}|{RESET}  Cores: {} Phys / {} Log  {CPU_COLOR}|{RESET}  Freq: {} MHz",
            temp_color,
            self.temperature_c,
            self.physical_cores,
            self.logical_cores,
            self.frequency_mhz
        )));

        lines
    }

    fn render_core_grid(&self, content_width: usize, num_cols: usize, col_gap: usize) -> Vec<BoxLine> {
        let total_cores = self.core_usages.len();
        if total_cores == 0 {
            return Vec::new();
        }

        let total_gaps = (num_cols - 1) * col_gap;
        let col_width = content_width.saturating_sub(total_gaps) / num_cols;
        let rows_per_col = (total_cores + num_cols - 1) / num_cols;

        let mut lines = Vec::with_capacity(rows_per_col);

        for row in 0..rows_per_col {
            let mut row_segments = Vec::with_capacity(num_cols);

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

            lines.push(BoxLine::Left(row_segments.join(&" ".repeat(col_gap))));
        }

        lines
    }

    fn get_temperature_c(components: &Components) -> f32 {
        components.iter()
            .find(|c| {
                let label = c.label().to_lowercase();
                label.contains("cpu") || label.contains("tctl") || label.contains("k10temp") || label.contains("coretemp")
            })
            .and_then(|c| c.temperature())
            .unwrap_or(0.0)
    }

    fn record_history_sample(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_history_sample) >= self.history_interval {
            self.usage_history.push_back(self.global_usage as f64);
            if self.usage_history.len() > self.max_history_len {
                self.usage_history.pop_front();
            }
            self.last_history_sample = now;
        }
    }
}