use std::io::{self, Write};
use std::collections::VecDeque;
use terminal_size::{terminal_size, Height, Width};

// ANSI Escape Codes
pub const RESET: &str = "\x1b[0m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const DARK_GRAY: &str = "\x1b[90m";

pub const BOLD: &str = "\x1b[1m";
pub const BOLD_RED: &str = "\x1b[1;31m";
pub const BOLD_GREEN: &str = "\x1b[1;32m";
pub const BOLD_YELLOW: &str = "\x1b[1;33m";
pub const BOLD_BLUE: &str = "\x1b[1;34m";
pub const BOLD_MAGENTA: &str = "\x1b[1;35m";
pub const BOLD_CYAN: &str = "\x1b[1;36m";

#[derive(Clone, Copy)]
pub enum ColorMode {
    DynamicThreshold,
    Fixed(&'static str),
    FixedRgb(u8, u8, u8),
}

pub enum Alignment {
    Left,
    Center,
    Right,
}

pub enum BoxLine {
    Left(String),
    Center(String),
    Right(String),
    Split(String, String),
}

pub struct BoxStyle {
    pub color_code: &'static str,
    pub title: String,
    pub width: usize,
}

/// Resolves the ANSI escape sequence based on ColorMode and current percentage.
pub fn resolve_color(mode: ColorMode, percent: f64) -> String {
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

/// Resolves an ANSI color escape sequence for temperature (°C).
/// Green under 70°C, Yellow from 70°C to 79°C, and Red at 80°C and above.
pub fn resolve_temp_color(temp_c: f32) -> &'static str {
    if temp_c >= 80.0 {
        RED
    } else if temp_c >= 70.0 {
        YELLOW
    } else {
        GREEN
    }
}

/// Resolves an ANSI color based on custom warning and critical temperature thresholds.
pub fn resolve_temp_color_custom(temp_c: f32, warn_threshold: f32, crit_threshold: f32) -> &'static str {
    if temp_c >= crit_threshold {
        RED
    } else if temp_c >= warn_threshold {
        YELLOW
    } else {
        GREEN
    }
}

/// Formats a percentage string with ANSI dynamic threshold coloring.
pub fn format_percent(percent: f64, width: usize, precision: usize) -> String {
    let color = resolve_color(ColorMode::DynamicThreshold, percent);
    format!("{color}{:width$.precision$}%{RESET}", percent, width = width, precision = precision)
}

/// Formats byte rates into human-readable transfer speeds
pub fn format_rate(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes_per_sec >= GB {
        format!("{:5.1} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:5.1} MB/s", bytes_per_sec / MB)
    } else if bytes_per_sec >= KB {
        format!("{:5.1} KB/s", bytes_per_sec / KB)
    } else {
        format!("{:5.0}  B/s", bytes_per_sec)
    }
}

/// Formats raw bytes into human-readable B, KB, MB, GB, or TB.
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.1} TB", b / TB)
    } else if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Measures printable length ignoring ANSI color escape sequences.
pub fn strip_ansi_len(s: &str) -> usize {
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

/// Renders a horizontal bar with an active fill and textured/dimmed track.
pub fn render_bar(percent: f64, width: usize, mode: ColorMode, track_char: char) -> String {
    let clamped = percent.clamp(0.0, 100.0);
    let filled_len = ((clamped / 100.0) * width as f64).round() as usize;
    let empty_len = width.saturating_sub(filled_len);

    let color = resolve_color(mode, clamped);
    let filled_str = "█".repeat(filled_len);

    let empty_str: String = std::iter::repeat(track_char).take(empty_len).collect();

    format!("{color}{filled_str}{empty_str}{RESET}")
}

/// Renders a sparkline histogram.
pub fn render_sparkline(history: &VecDeque<f64>, width: usize, mode: ColorMode, pad_char: char, ) -> String {
    const SPARK_CHARS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let mut result = String::with_capacity(width);

    if history.len() < width {
        let pad_len = width - history.len();
        let pad_str: String = std::iter::repeat(pad_char).take(pad_len).collect();
        result.push_str(&format!("{DARK_GRAY}{pad_str}{RESET}"));
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

/// Formats text according to an alignment within a given target width.
pub fn pad_aligned(text: &str, target_width: usize, align: Alignment) -> String {
    let visible_len = strip_ansi_len(text);
    let total_padding = target_width.saturating_sub(visible_len);

    match align {
        Alignment::Left => {
            let left_pad = 1;
            let right_pad = total_padding.saturating_sub(left_pad);
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        Alignment::Center => {
            let left_pad = total_padding / 2;
            let right_pad = total_padding.saturating_sub(left_pad);
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        Alignment::Right => {
            let right_pad = 1;
            let left_pad = total_padding.saturating_sub(right_pad);
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
    }
}

/// Splits text across the inner width: `left` against left edge, `right` against right edge.
pub fn justify_split(left: &str, right: &str, inner_width: usize) -> String {
    let left_len = strip_ansi_len(left);
    let right_len = strip_ansi_len(right);
    let available_middle = inner_width.saturating_sub(left_len + right_len + 2);

    format!(" {}{}{} ", left, " ".repeat(available_middle), right)
}

/// Renders lines into a rounded, colored box with a centered title.
pub fn render_box(lines: &[BoxLine], style: &BoxStyle) -> Vec<String> {
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
        let content = match line {
            BoxLine::Left(text) => pad_aligned(text, inner_width, Alignment::Left),
            BoxLine::Center(text) => pad_aligned(text, inner_width, Alignment::Center),
            BoxLine::Right(text) => pad_aligned(text, inner_width, Alignment::Right),
            BoxLine::Split(left, right) => justify_split(left, right, inner_width),
        };
        output.push(format!("{color}│{RESET}{}{color}│{RESET}", content));
    }

    output.push(format!(
        "{color}╰{}╯{RESET}",
        "─".repeat(width.saturating_sub(2))
    ));

    output
}

/// Combines two boxes side-by-side with an exact gap and line clearing.
pub fn box_columns(left_box: &[String], right_box: &[String], left_width: usize) -> Vec<String> {
    let max_lines = left_box.len().max(right_box.len());
    let blank_left = " ".repeat(left_width);

    let mut lines = Vec::new();

    for i in 0..max_lines {
        let left_str = left_box.get(i).map(|s| s.as_str()).unwrap_or(&blank_left);
        let right_str = right_box.get(i).map(|s| s.as_str()).unwrap_or("");

        lines.push(format!("{}  {}\x1b[K", left_str, right_str));
    }

    lines
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

/// Clears screen and homes the cursor to row 1, col 1
pub fn clear_and_home() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

/// Moves cursor to row 1, col 1 without clearing (prevents flicker)
pub fn cursor_home() {
    print!("\x1b[H");
    let _ = io::stdout().flush();
}

/// Hides the terminal cursor
pub fn hide_cursor() {
    print!("\x1b[?25l");
    let _ = io::stdout().flush();
}

/// Restores the terminal cursor (essential before exiting)
pub fn show_cursor() {
    print!("\x1b[?25h");
    let _ = io::stdout().flush();
}