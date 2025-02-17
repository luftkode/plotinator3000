use egui::{Vec2, Vec2b};
use egui_plot::{AxisHints, HPlacement, Legend, Plot};
use plot_util::{PlotData, Plots};

use crate::{mqtt::MqttData, plot::util};

use super::{axis_config::AxisConfig, plot_settings::PlotSettings, ClickDelta, PlotType};

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
/// * `click_delta` - State relating to pointer clicks on plots
#[allow(
    clippy::too_many_arguments,
    reason = "They are needed. Maybe a refactor could group some of them."
)]
pub fn paint_plots(
    ui: &mut egui::Ui,
    plots: &mut Plots,
    plot_settings: &PlotSettings,
    legend_cfg: &Legend,
    axis_cfg: &AxisConfig,
    link_group: egui::Id,
    line_width: f32,
    click_delta: &mut ClickDelta,
    mqtt_plots: &[MqttData],
) {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();
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
        x_axes.clone(),
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

    let mqtt_plot_area: Option<Plot<'_>> = if mqtt_plots.is_empty() {
        None
    } else {
        Some(build_plot_ui(
            "mqtt",
            ui.available_height(),
            legend_cfg.clone(),
            axis_cfg,
            x_axes,
            link_group,
        ))
    };

    fill_plots(
        ui,
        plot_components_list,
        line_width,
        plot_settings,
        click_delta,
        mqtt_plot_area,
        mqtt_plots,
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
    line_width: f32,
    plot_settings: &PlotSettings,
    click_delta: &mut ClickDelta,
    mqtt_plot_area: Option<Plot<'_>>,
    mqtt_plots: &[MqttData],
) {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();

    let (scroll, modifiers) = util::get_cursor_scroll_input(gui);
    let final_zoom_factor: Option<Vec2> = scroll.and_then(|s| util::set_zoom_factor(s, modifiers));

    for (ui, plot, ptype) in plot_components {
        ui.show(gui, |plot_ui| {
            if plot_ui.response().hovered() {
                if let Some(final_zoom_factor) = final_zoom_factor {
                    plot_ui.zoom_bounds_around_hovered(final_zoom_factor);
                }
            }
            let resp = plot_ui.response();
            if resp.clicked() {
                if plot_ui.ctx().input(|i| i.modifiers.shift) {
                    if let Some(pointer_coordinate) = plot_ui.pointer_coordinate() {
                        click_delta.set_next_click(pointer_coordinate, ptype);
                    }
                } else {
                    click_delta.reset();
                }
            }
            click_delta.ui(plot_ui, ptype);

            fill_plot(plot_ui, plot, line_width, plot_settings);
        });
    }
    if let Some(mqtt_plot_area) = mqtt_plot_area {
        mqtt_plot_area.show(gui, |plot_ui| {
            if plot_ui.response().hovered() {
                if let Some(final_zoom_factor) = final_zoom_factor {
                    plot_ui.zoom_bounds_around_hovered(final_zoom_factor);
                }
            }
            let resp = plot_ui.response();
            if resp.clicked() {
                if plot_ui.ctx().input(|i| i.modifiers.shift) {
                    if let Some(pointer_coordinate) = plot_ui.pointer_coordinate() {
                        click_delta.set_next_click(pointer_coordinate, PlotType::Hundreds);
                    }
                } else {
                    click_delta.reset();
                }
            }
            click_delta.ui(plot_ui, PlotType::Hundreds);
            let (x_lower, x_higher) = plot_util::extended_x_plot_bound(plot_ui.plot_bounds(), 0.1);
            for mp in mqtt_plots {
                if mp.data.len() < 2 {
                    // We don't plot less than two points. It's mostly because when the
                    // plotting starts, the auto-bounds causes a crash due to auto sizing
                    // a plot to 1 point and triggering an assert in egui_plot that the
                    // height and width of the bounds is greater than 0.0
                    continue;
                }
                plot_util::plot_raw_mqtt(
                    plot_ui,
                    &mp.topic,
                    &mp.data,
                    line_width,
                    (x_lower, x_higher),
                );
            }
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
fn fill_plot<'p>(
    plot_ui: &mut egui_plot::PlotUi<'p>,
    plot_data: &'p mut PlotData,
    line_width: f32,
    plot_settings: &'p PlotSettings,
) {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();
    // necessary because the raw plot points are not serializable
    // so they are skipped and initialized as None. So this
    // generates them from the raw_points (only needed once per session)
    plot_data
        .plots_as_mut()
        .iter_mut()
        .for_each(|p| p.build_raw_plot_points());

    plot_util::plot_lines(
        plot_ui,
        plot_settings.apply_filters(plot_data.plots()),
        line_width,
        plot_settings.mipmap_cfg(),
        plot_ui.ctx().used_size().x as usize,
    );

    plot_util::plot_labels(plot_ui, plot_data, &plot_settings.log_id_filter());
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
        .link_axis(link_group, Vec2b::new(axis_config.link_x(), false))
        .link_cursor(link_group, [axis_config.link_cursor_x(), false])
        .y_axis_min_width(50.0) // Adds enough margin for 5-digits
        .allow_boxed_zoom(true)
        .allow_zoom(false) // Manually implemented
        .allow_scroll(true)
}
