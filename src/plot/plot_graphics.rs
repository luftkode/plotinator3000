use egui_plot::{AxisHints, HPlacement, Legend, Plot};
use plot_util::{PlotData, Plots};

use super::{axis_config::AxisConfig, plot_settings::PlotSettings, PlotType};

/// Paints multiple plots based on the provided settings and configurations.
///
/// # Arguments
///
/// * `ui` - The egui UI to paint on.
/// * `plots` - The [`Plots`] struct containing plot data.
/// * `plot_settings` - Controls plot display.
/// * `legend_cfg` - Legend configuration.
/// * `axis_cfg` - For axis customization.
/// * `link_group` - An [`egui::Id`] for linking plots.
/// * `line_width` - The width of plot lines.
pub fn paint_plots(
    ui: &mut egui::Ui,
    plots: &mut Plots,
    plot_settings: &PlotSettings,
    legend_cfg: &Legend,
    axis_cfg: &mut AxisConfig,
    link_group: egui::Id,
    line_width: f32,
) {
    let plot_height = ui.available_height() / (plot_settings.total_plot_count() as f32);

    let x_axes = vec![AxisHints::new_x().formatter(crate::util::format_time)];

    let percentage_plot = build_plot_ui(
        "percentage",
        plot_height,
        legend_cfg.clone(),
        axis_cfg,
        x_axes.clone(),
        link_group,
    )
    .include_y(1.0)
    .y_axis_formatter(|y, _range| format!("{:.0}%", y.value * 100.0));

    let to_hundred_plot = build_plot_ui(
        "to_hundred",
        plot_height,
        legend_cfg.clone(),
        axis_cfg,
        x_axes.clone(),
        link_group,
    );
    let thousands_plot: Plot<'_> = build_plot_ui(
        "thousands",
        plot_height,
        legend_cfg.clone(),
        axis_cfg,
        x_axes,
        link_group,
    );
    let mut plot_components_list = Vec::with_capacity(plot_settings.total_plot_count().into());

    let Plots {
        percentage,
        one_to_hundred,
        thousands,
    } = plots;

    if plot_settings.display_percentage() {
        plot_components_list.push((percentage_plot, percentage, PlotType::Percentage));
    }

    if plot_settings.display_hundreds() {
        plot_components_list.push((to_hundred_plot, one_to_hundred, PlotType::Hundreds));
    }

    if plot_settings.display_thousands() {
        plot_components_list.push((thousands_plot, thousands, PlotType::Thousands));
    }

    fill_plots(
        ui,
        plot_components_list,
        axis_cfg,
        line_width,
        plot_settings,
    );
}

/// Iterates through and fills/paints all plots with their respective data.
///
/// # Arguments
///
/// * `gui` - The egui UI to paint on.
/// * `plot_components` - A vector of tuples containing [`Plot`], [`PlotData`], and [`PlotType`].
/// * `axis_config` - For axis customization.
/// * `line_width` - The width of plot lines.
/// * `plot_settings` - Controls which plots to display.
fn fill_plots(
    gui: &mut egui::Ui,
    plot_components: Vec<(Plot<'_>, &mut PlotData, PlotType)>,
    axis_config: &mut AxisConfig,
    line_width: f32,
    plot_settings: &PlotSettings,
) {
    for (ui, plot, ptype) in plot_components {
        ui.show(gui, |plot_ui| {
            fill_plot(
                plot_ui,
                (plot, ptype),
                axis_config,
                line_width,
                plot_settings,
            );
        });
    }
}

/// Fills and paints a single plot with its data.
///
/// # Arguments
///
/// * `plot_ui` - The plot UI to paint on.
/// * `plot` - A tuple containing [`PlotData`] and [`PlotType`].
/// * `axis_config` - For axis customization.
/// * `line_width` - The width of plot lines.
/// * `plot_settings` - Controls which plots to display.
fn fill_plot(
    plot_ui: &mut egui_plot::PlotUi,
    plot: (&mut PlotData, PlotType),
    axis_config: &mut AxisConfig,
    line_width: f32,
    plot_settings: &PlotSettings,
) {
    let (plot_data, plot_type) = plot;

    plot_util::plot_lines(
        plot_ui,
        plot_settings.apply_filters(plot_data.plots()),
        line_width,
        plot_settings.mipmap_cfg(),
        plot_ui.ctx().used_size().x as usize,
    );

    plot_util::plot_labels(plot_ui, plot_data, &plot_settings.log_id_filter());

    axis_config.handle_y_axis_lock(plot_ui, plot_type, |_| {});
}

/// Builds and configures a Plot UI (layout) with the specified settings.
///
/// # Arguments
///
/// * `name` - The name of the plot.
/// * `plot_height` - The height of the plot.
/// * `legend_cfg` - The legend configuration.
/// * `axis_config` - For axis customization.
/// * `x_axes` - A vector of [`AxisHints`] for x-axis configuration.
/// * `link_group` - An [`egui::Id`] for linking plots.
///
/// # Returns
///
/// A configured [`Plot`] instance.
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
        .y_axis_min_width(50.0) // Adds enough margin for 5-digits
}
