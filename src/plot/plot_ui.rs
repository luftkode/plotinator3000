use super::{axis_config::AxisConfig, plot_settings::PlotSettings};

// filter settings should be refactored out to be a standalone thing, maybe together with loaded_logs_ui
pub fn show_settings_grid(
    ui: &mut egui::Ui,
    axis_cfg: &mut AxisConfig,
    plot_settings: &mut PlotSettings,
) {
    ui.horizontal_wrapped(|ui| {
        plot_settings.show(ui, axis_cfg);
    });
}
