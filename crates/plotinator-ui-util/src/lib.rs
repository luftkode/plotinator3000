use egui::Color32;

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
