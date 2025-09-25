use egui::{Vec2, Vec2b};
use egui_plot::{AxisHints, HPlacement, Legend, Plot, PlotBounds};
use plotinator_plot_util::{PlotData, Plots};
use plotinator_ui_util::{PlotType, box_selection::BoxSelection};

use crate::{PlotMode, util};

use super::{ClickDelta, axis_config::AxisConfig, plot_settings::PlotSettings, x_axis_formatter};

/// Paints multiple plots based on the provided settings and configurations.
///
/// # Arguments
///
/// * `ui` - The egui UI to paint on.
/// * `reset_plot_bounds` - whether plot bounds should be reset.
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
    reset_plot_bounds: bool,
    plot_settings: &PlotSettings,
    legend_cfg: &Legend,
    axis_cfg: &AxisConfig,
    link_group: egui::Id,
    click_delta: &mut ClickDelta,
    box_selection: &mut BoxSelection,
    mode: PlotMode<'_>,
) {
    plotinator_macros::profile_function!();

    let x_axes = vec![AxisHints::new_x().formatter(x_axis_formatter::format_time)];

    match mode {
        PlotMode::Logs(plots) => {
            let plot_height = ui.available_height() / (plot_settings.total_plot_count() as f32);

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
            let mut plot_components_list =
                Vec::with_capacity(plot_settings.total_plot_count().into());

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
            fill_log_plots(
                ui,
                reset_plot_bounds,
                plot_components_list,
                plot_settings,
                click_delta,
                box_selection,
            );
        }
        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
        PlotMode::MQTT(mqtt_plots, set_auto_bounds) => {
            let mqtt_plot = build_plot_ui(
                "mqtt",
                ui.available_height(),
                legend_cfg.clone(),
                axis_cfg,
                x_axes,
                link_group,
            );
            crate::plot_mqtt::fill_mqtt_plots(
                ui,
                reset_plot_bounds,
                plot_settings.line_plot_settings().draw_mode(),
                click_delta,
                mqtt_plot,
                mqtt_plots,
                set_auto_bounds,
            );
        }
    }
}

/// Iterates through and fills/paints all plots with their respective data.
///
/// # Arguments
///
/// * `gui` - The egui UI to paint on.
/// * `reset_plot_bounds` - whether plot bounds should be reset.
/// * `plot_components` - A vector of tuples containing [`Plot`], [`PlotData`], and [`PlotType`].
/// * `plot_settings` - Controls which plots to display.
/// * `click_delta` - State relating to pointer clicks on plots
fn fill_log_plots(
    gui: &mut egui::Ui,
    reset_plot_bounds: bool,
    plot_components: Vec<(Plot<'_>, &mut PlotData, PlotType)>,
    plot_settings: &PlotSettings,
    click_delta: &mut ClickDelta,
    box_selection: &mut BoxSelection,
) {
    plotinator_macros::profile_function!();

    let (scroll, modifiers) = util::get_cursor_scroll_input(gui);
    let final_zoom_factor: Option<Vec2> = scroll.and_then(|s| util::set_zoom_factor(s, modifiers));

    for (ui, plot, ptype) in plot_components {
        ui.show(gui, |plot_ui| {
            let area_hovered = plot_ui.response().hovered();
            if area_hovered {
                box_selection.record_key_and_pointer_events(plot_ui, ptype);
            }

            if plot_settings.highlight(ptype) {
                plotinator_ui_util::highlight_plot_rect(plot_ui);
            }
            if area_hovered {
                if let Some(final_zoom_factor) = final_zoom_factor {
                    plot_ui.zoom_bounds_around_hovered(final_zoom_factor);
                }
            }

            if plot_ui.response().double_clicked() || reset_plot_bounds {
                let filter_plots = plot_settings.apply_filters(plot.plots());
                let mut max_bounds: Option<PlotBounds> = None;
                for fp in filter_plots {
                    let fp_max_bounds = fp.get_max_bounds();
                    if let Some(max_bounds) = &mut max_bounds {
                        max_bounds.merge(&fp_max_bounds);
                    } else {
                        max_bounds = Some(fp_max_bounds);
                    }
                }
                if let Some(mut max_bounds) = max_bounds {
                    // finally extend each bound by 10%
                    let margin_fraction = egui::Vec2::splat(0.1);
                    max_bounds.add_relative_margin_x(margin_fraction);
                    max_bounds.add_relative_margin_y(margin_fraction);
                    plot_ui.set_plot_bounds(max_bounds);
                }
            } else if plot_ui.response().clicked() {
                if plot_ui.ctx().input(|i| i.modifiers.shift) {
                    if let Some(pointer_coordinate) = plot_ui.pointer_coordinate() {
                        click_delta.set_next_click(pointer_coordinate, ptype);
                    }
                } else {
                    click_delta.reset();
                }
            }
            click_delta.ui(plot_ui, ptype);

            fill_plot(plot_ui, plot, plot_settings);
        });
    }
}

/// Fills and paints a single plot with its data.
///
/// # Arguments
///
/// * `plot_ui` - The plot UI to paint on.
/// * `plot_data` - [`PlotData`].
/// * `line_width` - The width of plot lines.
/// * `plot_settings` - Controls which plots to display.
fn fill_plot<'p>(
    plot_ui: &mut egui_plot::PlotUi<'p>,
    plot_data: &'p PlotData,
    plot_settings: &'p PlotSettings,
) {
    plotinator_macros::profile_function!();

    let line_plot_settings = plot_settings.line_plot_settings();

    plotinator_plot_util::plot_lines(
        plot_ui,
        plot_settings.apply_filters(plot_data.plots()),
        plot_settings.mipmap_cfg(),
        line_plot_settings.draw_mode(),
        plot_ui.ctx().used_size().x as usize,
    );

    plotinator_plot_util::plot_labels(plot_ui, plot_data, &plot_settings.log_id_filter());
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
        .label_formatter(plotinator_strfmt::format_label_ns)
        .link_axis(link_group, Vec2b::new(axis_config.link_x(), false))
        .link_cursor(link_group, [axis_config.link_cursor_x(), false])
        .y_axis_min_width(60.0) // Adds enough margin for 5-digits
        .allow_boxed_zoom(true)
        .allow_zoom(false) // Manually implemented
        .allow_scroll(true)
        .allow_double_click_reset(false) // Manually implemented
        .x_grid_spacer(x_axis_formatter::x_grid)
}
