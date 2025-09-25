use egui_plot::PlotBounds;

use super::{axis_config::AxisConfig, plot_settings::PlotSettings};

// filter settings should be refactored out to be a standalone thing, maybe together with loaded_logs_ui
pub fn show_settings_grid(
    ui: &mut egui::Ui,
    axis_cfg: &mut AxisConfig,
    plot_settings: &mut PlotSettings,
    plots: &plotinator_plot_util::Plots,
    selected_box: Option<PlotBounds>,
) {
    ui.horizontal_wrapped(|ui| {
        plot_settings.show(ui, axis_cfg, plots, selected_box);
    });
}
