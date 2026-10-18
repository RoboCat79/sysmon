use std::collections::VecDeque;
use std::time::{Duration, Instant};
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Nvml;

use crate::MAX_HISTORY_LEN;
use crate::REFRESH_SPARKLINE;
use crate::ui::{
    render_bar, render_box, render_sparkline, format_percent, format_bytes,
    BoxLine, BoxStyle, ColorMode, resolve_temp_color, strip_ansi_len,
    RESET, MAGENTA, BOLD, BOLD_YELLOW,
};

const GPU_COLOR: &str = MAGENTA;

#[derive(Debug)]
pub struct Metrics {
    pub name: String,
    pub gpu_util: f64,
    pub memory_util: f64,
    pub vram_total: u64,
    pub vram_used: u64,
    pub vram_free: u64,
    pub temperature_c: f32,
    pub power_watts: f32,
    pub fan_percent: u32,
}

pub struct GPU {
    nvml: Option<Nvml>,
    pub metrics: Option<Metrics>,

    pub usage_history: VecDeque<f64>,
    pub vram_history: VecDeque<f64>,
    pub max_history_len: usize,
    pub history_interval: Duration,
    pub last_history_sample: Instant,
}

impl GPU {
    pub fn new() -> Self {
        let nvml = Nvml::init().ok();
        let max_history_len = MAX_HISTORY_LEN;

        let mut gpu = Self {
            nvml,
            metrics: None,
            usage_history: VecDeque::with_capacity(max_history_len),
            vram_history: VecDeque::with_capacity(max_history_len),
            max_history_len,
            history_interval: REFRESH_SPARKLINE,
            last_history_sample: Instant::now() - Duration::from_millis(500),
        };

        gpu.update();
        gpu
    }

    pub fn update(&mut self) {
        if let Some(ref nvml) = self.nvml {
            if let Ok(device) = nvml.device_by_index(0) {
                let name = device.name().unwrap_or_else(|_| "NVIDIA GPU".to_string());
                let util = device.utilization_rates().ok();
                let gpu_util = util.as_ref().map(|u| u.gpu as f64).unwrap_or(0.0);

                let mem_info = device.memory_info().ok();
                let vram_total = mem_info.as_ref().map(|m| m.total).unwrap_or(0);
                let vram_used = mem_info.as_ref().map(|m| m.used).unwrap_or(0);
                let vram_free = mem_info.as_ref().map(|m| m.free).unwrap_or(0);
                let memory_util = if vram_total > 0 {
                    (vram_used as f64 / vram_total as f64) * 100.0
                } else {
                    0.0
                };

                let temp = device
                    .temperature(TemperatureSensor::Gpu)
                    .map(|t| t as f32)
                    .unwrap_or(0.0);

                let power_watts = device
                    .power_usage()
                    .map(|mw| mw as f32 / 1000.0)
                    .unwrap_or(0.0);

                let fan_percent = device.fan_speed(0).unwrap_or(0);

                self.metrics = Some(Metrics {
                    name,
                    gpu_util,
                    memory_util,
                    vram_total,
                    vram_used,
                    vram_free,
                    temperature_c: temp,
                    power_watts,
                    fan_percent,
                });
            }
        }

        self.record_history_sample();
    }

    pub fn display(&self, width: usize) -> Vec<String> {
        let inner_width = width.saturating_sub(2);
        let lines = self.render_lines(inner_width);

        let title = format!("{BOLD}{GPU_COLOR}{}{RESET}", self
            .metrics
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "GPU".to_string())
        );


        let style = BoxStyle {
            color_code: GPU_COLOR,
            title,
            width,
        };

        render_box(&lines, &style)
    }

    pub fn render_lines(&self, inner_width: usize) -> Vec<BoxLine> {
        let content_width = inner_width.saturating_sub(2);
        let mut lines = Vec::new();

        let metrics = match &self.metrics {
            Some(m) => m,
            None => {
                lines.push(BoxLine::Center("No Dedicated GPU Detected".to_string()));
                return lines;
            }
        };

        let gpu_pct_str = format_percent(metrics.gpu_util, 5, 1);
        let gpu_prefix = format!("{BOLD_YELLOW}GPU Usage:{RESET}  {} ", gpu_pct_str);
        let gpu_prefix_len = strip_ansi_len(&gpu_prefix);
        let dynamic_bar_width = content_width.saturating_sub(gpu_prefix_len).max(5);

        let gpu_bar = render_bar(
            metrics.gpu_util,
            dynamic_bar_width,
            ColorMode::DynamicThreshold,
            '░',
        );
        lines.push(BoxLine::Left(format!("{}{}", gpu_prefix, gpu_bar)));

        let trend_label = format!("{BOLD_YELLOW}GPU Trend:{RESET}  ");
        let trend_label_len = strip_ansi_len(&trend_label);
        let trend_pad = gpu_prefix_len.saturating_sub(trend_label_len);

        let trend = render_sparkline(
            &self.usage_history,
            dynamic_bar_width,
            ColorMode::DynamicThreshold,
            '-',
        );
        lines.push(BoxLine::Left(format!(
            "{}{}{}",
            trend_label,
            " ".repeat(trend_pad),
            trend
        )));

        lines.push(BoxLine::Left(String::new()));

        let vram_pct_str = format_percent(metrics.memory_util, 5, 1);
        let vram_prefix = format!("{BOLD_YELLOW}VRAM Usage:{RESET} {} ", vram_pct_str);
        let vram_prefix_len = strip_ansi_len(&vram_prefix);
        let vram_bar_width = content_width.saturating_sub(vram_prefix_len).max(5);

        let vram_bar = render_bar(
            metrics.memory_util,
            vram_bar_width,
            ColorMode::DynamicThreshold,
            '░',
        );
        lines.push(BoxLine::Left(format!("{}{}", vram_prefix, vram_bar)));

        let vram_trend_label = format!("{BOLD_YELLOW}VRAM Trend:{RESET} ");
        let vram_trend_len = strip_ansi_len(&vram_trend_label);
        let vram_trend_pad = vram_prefix_len.saturating_sub(vram_trend_len);

        let vram_sparkline = render_sparkline(
            &self.vram_history,
            vram_bar_width,
            ColorMode::DynamicThreshold,
            '-',
        );
        lines.push(BoxLine::Left(format!(
            "{}{}{}",
            vram_trend_label,
            " ".repeat(vram_trend_pad),
            vram_sparkline
        )));

        lines.push(BoxLine::Left(format!(
            "Used: {}  {GPU_COLOR}|{RESET}  Free: {}  {GPU_COLOR}|{RESET}  Total: {}",
            format_bytes(metrics.vram_used),
            format_bytes(metrics.vram_free),
            format_bytes(metrics.vram_total),
        )));

        lines.push(BoxLine::Left(String::new()));

        let temp_color = resolve_temp_color(metrics.temperature_c);
        lines.push(BoxLine::Left(format!(
            "Temp: {}{:.0}°C{RESET}  {GPU_COLOR}|{RESET}  Power: {:.0} W  {GPU_COLOR}|{RESET}  Fan: {}%",
            temp_color,
            metrics.temperature_c,
            metrics.power_watts,
            metrics.fan_percent
        )));

        lines
    }

    fn record_history_sample(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_history_sample) >= self.history_interval {
            let (gpu_util, vram_util) = match &self.metrics {
                Some(m) => (m.gpu_util, m.memory_util),
                None => (0.0, 0.0),
            };

            self.usage_history.push_back(gpu_util);
            if self.usage_history.len() > self.max_history_len {
                self.usage_history.pop_front();
            }

            self.vram_history.push_back(vram_util);
            if self.vram_history.len() > self.max_history_len {
                self.vram_history.pop_front();
            }

            self.last_history_sample = now;
        }
    }
}