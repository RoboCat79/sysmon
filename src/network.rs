use std::collections::VecDeque;
use std::time::{Duration, Instant};
use sysinfo::Networks;

use crate::MAX_HISTORY_LEN;
use crate::REFRESH_SPARKLINE;
use crate::ui::{
    render_bar, render_box, render_sparkline, format_bytes, format_rate,
    BoxLine, BoxStyle, ColorMode, strip_ansi_len,
    RESET, CYAN, BOLD, BOLD_YELLOW,
};

const NETWORK_COLOR: &str = CYAN;

#[derive(Debug)]
pub struct Network {
    pub primary_interface: String,

    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,

    pub rx_rate_bps: f64,
    pub tx_rate_bps: f64,

    pub peak_rx_bps: f64,
    pub peak_tx_bps: f64,

    last_rx_raw: u64,
    last_tx_raw: u64,
    last_tick: Instant,

    pub rx_history: VecDeque<f64>,
    pub tx_history: VecDeque<f64>,
    pub max_history_len: usize,
    pub history_interval: Duration,
    pub last_history_sample: Instant,
}

impl Network {
    pub fn new(networks: &Networks) -> Self {
        let max_history_len = MAX_HISTORY_LEN;

        let primary_interface = networks
            .keys()
            .find(|&name| name != "lo")
            .cloned()
            .unwrap_or_else(|| "lo".to_string());

        let (rx_init, tx_init) = if let Some(data) = networks.get(&primary_interface) {
            (data.total_received(), data.total_transmitted())
        } else {
            (0, 0)
        };

        Self {
            primary_interface,
            total_rx_bytes: rx_init,
            total_tx_bytes: tx_init,
            rx_rate_bps: 0.0,
            tx_rate_bps: 0.0,
            peak_rx_bps: 1_048_576.0,
            peak_tx_bps: 1_048_576.0,
            last_rx_raw: rx_init,
            last_tx_raw: tx_init,
            last_tick: Instant::now(),
            rx_history: VecDeque::with_capacity(max_history_len),
            tx_history: VecDeque::with_capacity(max_history_len),
            max_history_len,
            history_interval: REFRESH_SPARKLINE,
            last_history_sample: Instant::now() - Duration::from_millis(500),
        }
    }

    pub fn update(&mut self, networks: &Networks) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick).as_secs_f64().max(0.001);

        if self.primary_interface == "lo" || !networks.contains_key(&self.primary_interface) {
            if let Some(name) = networks.keys().find(|&n| n != "lo") {
                self.primary_interface = name.clone();
            }
        }

        if let Some(data) = networks.get(&self.primary_interface) {
            let current_rx = data.total_received();
            let current_tx = data.total_transmitted();

            self.total_rx_bytes = current_rx;
            self.total_tx_bytes = current_tx;

            let delta_rx = current_rx.saturating_sub(self.last_rx_raw);
            let delta_tx = current_tx.saturating_sub(self.last_tx_raw);

            self.rx_rate_bps = delta_rx as f64 / elapsed;
            self.tx_rate_bps = delta_tx as f64 / elapsed;

            self.last_rx_raw = current_rx;
            self.last_tx_raw = current_tx;
            self.last_tick = now;
        }

        self.record_history_sample();
    }

    pub fn display(&self, width: usize) -> Vec<String> {
        let inner_width = width.saturating_sub(2);
        let lines = self.render_lines(inner_width);

        let title = format!("{BOLD}{NETWORK_COLOR}Network ({}){RESET}", self.primary_interface);

        let style = BoxStyle {
            color_code: NETWORK_COLOR,
            title,
            width,
        };

        render_box(&lines, &style)
    }

    pub fn render_lines(&self, inner_width: usize) -> Vec<BoxLine> {
        let content_width = inner_width.saturating_sub(2);
        let mut lines = Vec::new();

        let rx_rate_str = format_rate(self.rx_rate_bps);
        let rx_prefix = format!("{BOLD_YELLOW}Down:{RESET} {} ", rx_rate_str);
        let rx_prefix_len = strip_ansi_len(&rx_prefix);
        let rx_bar_width = content_width.saturating_sub(rx_prefix_len).max(5);

        let rx_percent = (self.rx_rate_bps / self.peak_rx_bps * 100.0).clamp(0.0, 100.0);
        let rx_bar = render_bar(
            rx_percent,
            rx_bar_width,
            ColorMode::Fixed(CYAN),
            '░',
        );
        lines.push(BoxLine::Left(format!("{}{}", rx_prefix, rx_bar)));

        let rx_trend_label = format!("{BOLD_YELLOW}Down Trend: {RESET} ");
        let rx_trend_len = strip_ansi_len(&rx_trend_label);
        let rx_trend_pad = rx_prefix_len.saturating_sub(rx_trend_len);

        let rx_sparkline = render_sparkline(
            &self.rx_history,
            rx_bar_width,
            ColorMode::Fixed(CYAN),
            '-',
        );
        lines.push(BoxLine::Left(format!(
            "{}{}{}",
            rx_trend_label,
            " ".repeat(rx_trend_pad),
            rx_sparkline
        )));

        lines.push(BoxLine::Left(format!(
            "Total: {}  {NETWORK_COLOR}|{RESET}  Peak: {}",
            format_bytes(self.total_rx_bytes),
            format_rate(self.peak_rx_bps)
        )));

        lines.push(BoxLine::Left(String::new()));

        let tx_rate_str = format_rate(self.tx_rate_bps);
        let tx_prefix = format!("{BOLD_YELLOW}Up:  {RESET} {} ", tx_rate_str);
        let tx_prefix_len = strip_ansi_len(&tx_prefix);
        let tx_bar_width = content_width.saturating_sub(tx_prefix_len).max(5);

        let tx_percent = (self.tx_rate_bps / self.peak_tx_bps * 100.0).clamp(0.0, 100.0);
        let tx_bar = render_bar(
            tx_percent,
            tx_bar_width,
            ColorMode::Fixed(CYAN),
            '░',
        );
        lines.push(BoxLine::Left(format!("{}{}", tx_prefix, tx_bar)));

        let tx_trend_label = format!("{BOLD_YELLOW}Up Trend: {RESET} ");
        let tx_trend_len = strip_ansi_len(&tx_trend_label);
        let tx_trend_pad = tx_prefix_len.saturating_sub(tx_trend_len);

        let tx_sparkline = render_sparkline(
            &self.tx_history,
            tx_bar_width,
            ColorMode::Fixed(CYAN),
            '-',
        );
        lines.push(BoxLine::Left(format!(
            "{}{}{}",
            tx_trend_label,
            " ".repeat(tx_trend_pad),
            tx_sparkline
        )));

        lines.push(BoxLine::Left(format!(
            "Total: {}  {NETWORK_COLOR}|{RESET}  Peak: {}",
            format_bytes(self.total_tx_bytes),
            format_rate(self.peak_tx_bps)
        )));

        lines
    }

    fn record_history_sample(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_history_sample) >= self.history_interval {
            self.rx_history.push_back(self.rx_rate_bps / 1024.0);
            if self.rx_history.len() > self.max_history_len {
                self.rx_history.pop_front();
            }

            self.tx_history.push_back(self.tx_rate_bps / 1024.0);
            if self.tx_history.len() > self.max_history_len {
                self.tx_history.pop_front();
            }

            self.last_history_sample = now;
        }
    }
}