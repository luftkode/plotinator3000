use std::sync::atomic::{AtomicUsize, Ordering};

use egui::Color32;
use serde::{Deserialize, Serialize};

pub mod box_selection;
pub mod date_editor;
pub mod number_editor;

/// Where does the plot values typically fit within, e.g. RPM measurements will probably be in the thousands, while a duty cycle will be in percentage.
#[derive(Debug, strum_macros::Display, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ExpectedPlotRange {
    /// For plots where the value is 0.0-1.0 and corresponds to percentage 0-100%
    Percentage,
    Hundreds,
    Thousands,
}

/// Selects between the colors based on the current UI theme
#[must_use]
pub fn theme_color(ui: &egui::Ui, dark: Color32, light: Color32) -> Color32 {
    match ui.ctx().theme() {
        egui::Theme::Dark => dark,
        egui::Theme::Light => light,
    }
}

/// Selects between the colors based on the current Plot UI theme (same as above)
#[must_use]
pub fn plot_theme_color(ui: &egui_plot::PlotUi, dark: Color32, light: Color32) -> Color32 {
    match ui.ctx().theme() {
        egui::Theme::Dark => dark,
        egui::Theme::Light => light,
    }
}

/// Highlights a UI rectangle element by drawing a thicker boundary and making the inner content slightly brigher/darker depending on the theme
pub fn highlight_plot_rect(ui: &egui_plot::PlotUi) {
    let rect = ui.response().rect;
    ui.ctx().debug_painter().rect_stroke(
        rect,
        egui::CornerRadius::same(2),
        egui::Stroke::new(3.0, plot_theme_color(ui, Color32::WHITE, Color32::BLACK)),
        egui::StrokeKind::Inside,
    );
    ui.ctx().debug_painter().rect_filled(
        rect,
        egui::CornerRadius::same(1),
        plot_theme_color(
            ui,
            Color32::from_rgba_unmultiplied_const(60, 60, 60, 80), // slightly brighter
            Color32::from_rgba_unmultiplied_const(180, 180, 180, 80), // slightly darker
        ),
    );
}

/// Formats a large number into a more human-readable string.
/// - Numbers under 1 million will be formatted with comma separators (e.g., 950,123).
/// - Numbers 1 million and over will be formatted with two decimal places (e.g., 1.21 M).
///
/// # Arguments
///
/// * `num` - The number to format.
///
/// # Returns
///
/// A formatted `String`.
pub fn format_large_number(num: u32) -> String {
    if num < 1_000_000 {
        // Format with comma separators for thousands
        let s = num.to_string();
        let mut result = String::with_capacity(s.len() + (s.len() - 1) / 3);
        // Iterate in reverse to place commas every three digits
        for (count, ch) in s.chars().rev().enumerate() {
            if count > 0 && count % 3 == 0 {
                result.push(',');
            }
            result.push(ch);
        }
        result.chars().rev().collect()
    } else {
        let millions = num as f64 / 1_000_000.0;
        format!("{millions:.2} M")
    }
}

pub fn auto_color_plot_area(range: ExpectedPlotRange) -> Color32 {
    let i = match range {
        ExpectedPlotRange::Percentage => {
            static COLOR_COUNTER_PERCENTAGE: AtomicUsize = AtomicUsize::new(0);
            COLOR_COUNTER_PERCENTAGE.fetch_add(1, Ordering::Relaxed)
        }
        ExpectedPlotRange::Hundreds => {
            static COLOR_COUNTER_HUNDREDS: AtomicUsize = AtomicUsize::new(0);
            COLOR_COUNTER_HUNDREDS.fetch_add(1, Ordering::Relaxed)
        }
        ExpectedPlotRange::Thousands => {
            static COLOR_COUNTER_THOUSANDS: AtomicUsize = AtomicUsize::new(0);
            COLOR_COUNTER_THOUSANDS.fetch_add(1, Ordering::Relaxed)
        }
    };
    get_color(i)
}

fn hue_from_golden_ratio(i: usize) -> f32 {
    let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
    i as f32 * golden_ratio
}

fn get_color(i: usize) -> Color32 {
    let h = hue_from_golden_ratio(i);
    egui::epaint::Hsva::new(h, 0.85, 0.5, 1.0).into()
}

/// Color for drawing on a map
pub fn auto_terrain_safe_color() -> Color32 {
    static COLOR_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let i = COLOR_COUNTER.fetch_add(1, Ordering::Relaxed);
    get_terrain_safe_color(i)
}

fn get_terrain_safe_color(i: usize) -> Color32 {
    let mut h = hue_from_golden_ratio(i) % 1.;

    // Skip terrain-like hues (green/yellow range: ~80°–150°)
    if (0.22..=0.42).contains(&h) {
        h = (h + 0.25) % 1.0;
    }

    // High contrast values for natural backgrounds
    let s = 0.9;
    let v = 0.95;

    egui::epaint::Hsva::new(h, s, v, 1.0).into()
}

/// Color for invalid/error data - purple for visibility
#[must_use]
pub fn invalid_data_color() -> Color32 {
    Color32::from_rgb(200, 100, 255)
}

/// Get the next unique ID for a log
#[must_use]
pub fn next_log_id() -> usize {
    static LOG_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);
    LOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}
