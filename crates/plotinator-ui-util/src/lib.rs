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
            Color32::from_rgba_unmultiplied(60, 60, 60, 80), // slightly brighter
            Color32::from_rgba_unmultiplied(180, 180, 180, 80), // slightly darker
        ),
    );
}

pub fn auto_color(auto_color_idx: &mut usize) -> Color32 {
    // source: https://docs.rs/egui_plot/0.29.0/src/egui_plot/plot_ui.rs.html#21
    // should be replaced/updated if they improve their implementation or provide a public API for this
    let i = *auto_color_idx;
    *auto_color_idx += 1;
    let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
    let h = i as f32 * golden_ratio;
    egui::epaint::Hsva::new(h, 0.85, 0.5, 1.0).into() // TODO(emilk): OkLab or some other perspective color space
}
