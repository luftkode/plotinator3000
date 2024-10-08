use egui::RichText;
use egui_plot::{AxisHints, HPlacement, Legend, Plot, PlotPoint};
use plot_util::{MipMapConfiguration, PlotData, Plots};

use super::{
    axis_config::AxisConfig, play_state::playback_update_plot, plot_settings::PlotSettings,
    PlotType,
};

#[allow(clippy::too_many_arguments)]
pub fn paint_plots(
    ui: &mut egui::Ui,
    plots: &Plots,
    plot_settings: &PlotSettings,
    legend_cfg: &Legend,
    axis_cfg: &mut AxisConfig,
    link_group: Option<egui::Id>,
    line_width: f32,
    timer: Option<f64>,
    is_reset_pressed: bool,
    x_min_max: Option<(f64, f64)>,
) {
    let plot_height = ui.available_height() / (plot_settings.total_plot_count() as f32);

    let plot_graphics = build_all_plot_uis(
        plot_height,
        legend_cfg,
        axis_cfg,
        link_group.expect("uninitialized link group id"),
    );
    let mut plot_components_list = Vec::with_capacity(plot_settings.total_plot_count().into());
    for (p_graphic, p_type) in plot_graphics {
        match p_type {
            PlotType::Percentage => {
                if plot_settings.display_percentage() {
                    plot_components_list.push((p_graphic, plots.percentage(), p_type));
                }
            }
            PlotType::Hundreds => {
                if plot_settings.display_hundreds() {
                    plot_components_list.push((p_graphic, plots.one_to_hundred(), p_type));
                }
            }
            PlotType::Thousands => {
                if plot_settings.display_thousands() {
                    plot_components_list.push((p_graphic, plots.thousands(), p_type));
                }
            }
        }
    }
    fill_plots(
        ui,
        plot_components_list,
        axis_cfg,
        line_width,
        timer,
        is_reset_pressed,
        x_min_max,
        &plot_settings.plot_name_filter(),
        &plot_settings.log_id_filter(),
        plot_settings.mipmap_cfg(),
    );
}

#[allow(clippy::too_many_arguments)]
/// Iterate and fill/paint all plots with plot data
fn fill_plots(
    gui: &mut egui::Ui,
    plot_components: Vec<(Plot<'_>, &PlotData, PlotType)>,
    axis_config: &mut AxisConfig,
    line_width: f32,
    timer: Option<f64>,
    is_reset_pressed: bool,
    x_min_max: Option<(f64, f64)>,
    plot_name_filter: &[&str],
    plot_id_filter: &[usize],
    mipmap_cfg: MipMapConfiguration,
) {
    for (ui, plot, ptype) in plot_components {
        ui.show(gui, |plot_ui| {
            fill_plot(
                plot_ui,
                (plot, ptype),
                axis_config,
                line_width,
                timer,
                is_reset_pressed,
                x_min_max,
                plot_name_filter,
                plot_id_filter,
                mipmap_cfg,
            );
        });
    }
}

#[allow(clippy::too_many_arguments)]
/// Iterate and fill/paint a plot with plot data
fn fill_plot(
    plot_ui: &mut egui_plot::PlotUi,
    plot: (&PlotData, PlotType),
    axis_config: &mut AxisConfig,
    line_width: f32,
    timer: Option<f64>,
    is_reset_pressed: bool,
    x_min_max: Option<(f64, f64)>,
    name_filter: &[&str],
    id_filter: &[usize],
    mipmap_cfg: MipMapConfiguration,
) {
    let (plot_data, plot_type) = plot;

    plot_util::plot_lines(
        plot_ui,
        plot_data.plots(),
        name_filter,
        id_filter,
        line_width,
        mipmap_cfg,
        plot_ui.ctx().used_size().x as usize,
    );

    plot_util::plot_labels(plot_ui, plot_data, id_filter);

    playback_update_plot(
        timer,
        plot_ui,
        is_reset_pressed,
        x_min_max.unwrap_or_default().0,
    );
    axis_config.handle_y_axis_lock(plot_ui, plot_type, |plot_ui| {
        playback_update_plot(
            timer,
            plot_ui,
            is_reset_pressed,
            x_min_max.unwrap_or_default().0,
        );
    });
}

/// Build/configure the plot UI/windows
fn build_all_plot_uis<'p>(
    plot_height: f32,
    legend_cfg: &Legend,
    axis_config: &AxisConfig,
    link_group: egui::Id,
) -> [(Plot<'p>, PlotType); 3] {
    let x_axes = vec![AxisHints::new_x().formatter(crate::util::format_time)];

    let percentage_plot = build_plot_ui(
        "percentage",
        plot_height,
        legend_cfg.clone(),
        axis_config,
        x_axes.clone(),
        link_group,
    )
    .include_y(1.0)
    .y_axis_formatter(|y, _range| format!("{:.0}%", y.value * 100.0));

    let to_hundred = build_plot_ui(
        "to_hundred",
        plot_height,
        legend_cfg.clone(),
        axis_config,
        x_axes.clone(),
        link_group,
    );
    let thousands: Plot<'_> = build_plot_ui(
        "thousands",
        plot_height,
        legend_cfg.clone(),
        axis_config,
        x_axes,
        link_group,
    );
    [
        (percentage_plot, PlotType::Percentage),
        (to_hundred, PlotType::Hundreds),
        (thousands, PlotType::Thousands),
    ]
}

fn build_plot_ui<'a>(
    name: &str,
    plot_height: f32,
    legend_cfg: Legend,
    axis_config: &AxisConfig,
    x_axes: Vec<AxisHints<'a>>,
    link_group: egui::Id,
) -> Plot<'a> {
    Plot::new(name)
        .legend(legend_cfg)
        .height(plot_height)
        .show_axes(axis_config.show_axes())
        .show_grid(axis_config.show_grid())
        .y_axis_position(HPlacement::Right)
        .include_y(0.0)
        .custom_x_axes(x_axes)
        .label_formatter(crate::util::format_label_ns)
        .link_axis(link_group, axis_config.link_x(), false)
        .link_cursor(link_group, axis_config.link_cursor_x(), false)
}
