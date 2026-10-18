use sysinfo::Disks;
use crate::ui::{render_bar, render_box, format_percent, format_bytes, BoxLine, BoxStyle, ColorMode, strip_ansi_len, RESET, YELLOW, BOLD, BOLD_YELLOW, BOLD_CYAN};

const STORAGE_COLOR: &str = YELLOW;

pub struct Disk {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: f64,
}
pub struct Storage {
    pub disks: Vec<Disk>,
}

impl Storage {
    pub fn new() -> Self {
        let mut storage = Self { disks: Vec::new() };
        storage.update();
        storage
    }

    pub fn update(&mut self) {
        let disks = Disks::new_with_refreshed_list();
        self.disks = disks
            .iter()
            .map(|d| {
                let total = d.total_space();
                let available = d.available_space();
                let used = total.saturating_sub(available);
                let percent = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                Disk {
                    name: d.name().to_string_lossy().to_string(),
                    mount_point: d.mount_point().to_string_lossy().to_string(),
                    total_bytes: total,
                    used_bytes: used,
                    usage_percent: percent,
                }
            })
            .collect();
    }

    pub fn display(&self, width: usize) -> Vec<String> {
        let inner_width = width.saturating_sub(2);
        let lines = self.render_lines(inner_width);

        let title = match self.get_primary_device_name() {
            Some(dev) if !dev.is_empty() => format!("{BOLD}{STORAGE_COLOR}Storage Drives ({}){RESET}", dev),
            _ => "Storage Drives".to_string(),
        };

        let style = BoxStyle {
            color_code: STORAGE_COLOR,
            title,
            width,
        };

        render_box(&lines, &style)
    }

    fn render_lines(&self, inner_width: usize) -> Vec<BoxLine> {
        let content_width = inner_width.saturating_sub(2);
        let mut lines = Vec::new();

        let displayed_disks = self.disks.iter();
        let max_mount_len = self
            .disks
            .iter()
            .map(|d| d.mount_point.chars().count())
            .max()
            .unwrap_or(4)
            .max(6);

        for disk in &self.disks {
            let mount_str = format!("{BOLD_CYAN}{:<width$}{RESET}", disk.mount_point, width = max_mount_len);

            let pct_str = format_percent(disk.usage_percent, 5, 1);

            let prefix = format!("{}  {} ", mount_str, pct_str);
            let prefix_len = strip_ansi_len(&prefix);

            let bytes_summary = format!(
                "{:>8} / {}",
                format_bytes(disk.used_bytes),
                format_bytes(disk.total_bytes)
            );
            let summary_len = strip_ansi_len(&bytes_summary);

            let bar_space = content_width.saturating_sub(prefix_len + summary_len + 1);
            let disk_bar = render_bar(
                disk.usage_percent,
                bar_space.max(4),
                ColorMode::DynamicThreshold,
                '░',
            );

            lines.push(BoxLine::Left(format!(
                "{}{} {}",
                prefix, disk_bar, bytes_summary
            )));
        }

        lines.push(BoxLine::Left(String::new())); // THIS IS A BUFFER TO MATCH THE HEIGHT OF THE CPU COLUMN
        lines.push(BoxLine::Left(String::new()));

        lines
    }

    fn get_primary_device_name(&self) -> Option<&str> {
        self.disks
            .iter()
            .find(|d| d.mount_point == "/")
            .or_else(|| self.disks.first())
            .map(|d| d.name.as_str())
    }
}