use egui::{Color32, Ui};

/// Selects between the colors based on the current UI theme
#[must_use]
pub fn theme_color(ui: &Ui, dark: Color32, light: Color32) -> Color32 {
    match ui.ctx().theme() {
        egui::Theme::Dark => dark,
        egui::Theme::Light => light,
    }
}
