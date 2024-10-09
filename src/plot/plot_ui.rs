use super::{axis_config::AxisConfig, plot_settings::PlotSettings};

// filter settings should be refactored out to be a standalone thing, maybe together with loaded_logs_ui
pub fn show_settings_grid(
    ui: &mut egui::Ui,
    line_width: &mut f32,
    axis_cfg: &mut AxisConfig,
    plot_settings: &mut PlotSettings,
) {
    egui::Grid::new("settings").show(ui, |ui| {
        ui.label("Line width");
        ui.add(
            egui::DragValue::new(line_width)
                .speed(0.02)
                .range(0.5..=20.0),
        );

        ui.horizontal_top(|ui| {
            axis_cfg.toggle_axis_cfg_ui(ui);
            ui.label("|");
            plot_settings.show(ui);
        });
    });
}
