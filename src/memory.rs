use sysinfo::System;

use crate::ui::{
    render_bar, render_box, render_sparkline, print_columns, format_percent, format_bytes,
    BoxLine, BoxStyle, ColorMode, Alignment, resolve_temp_color, strip_ansi_len,
    RESET, RED, GREEN, YELLOW, BLUE, MAGENTA, CYAN, DARK_GRAY,
    clear_and_home, cursor_home, hide_cursor, show_cursor
};

#[derive(Debug)]
pub struct Memory {
    pub total_ram: u64,
    pub total_swap: u64,

    pub used_ram: u64,
    pub used_swap: u64,
}

impl Memory {
    // Read stats from the system and create a new Memory instance
    pub fn new(sys: &System) -> Self {
        Self {
            total_ram: sys.total_memory(),
            total_swap: sys.total_swap(),
            used_ram: sys.used_memory(),
            used_swap: sys.used_swap(),
        }
    }

    pub fn update(&mut self, sys: &System) {
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