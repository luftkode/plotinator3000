use egui::RichText;
use egui_plot::{AxisHints, HPlacement, Legend, Plot, PlotPoint};
use plot_util::{PlotData, Plots};

use super::{axis_config::AxisConfig, play_state::playback_update_plot, PlotType};

#[allow(clippy::too_many_arguments)]
pub fn paint_plots(
    ui: &mut egui::Ui,
    total_plot_count: u8,
    legend_cfg: &Legend,
    axis_cfg: &mut AxisConfig,
    link_group: Option<egui::Id>,
    plot_wrapper: PlotWrapperHelper<'_>,
    line_width: f32,
    timer: Option<f64>,
    is_reset_pressed: bool,
    x_min_max: Option<(f64, f64)>,
    plot_name_filter: &[&str],
) {
    let plot_height = ui.available_height() / (total_plot_count as f32);

    let plot_graphics = build_all_plot_uis(
        plot_height,
        legend_cfg,
        axis_cfg,
        link_group.expect("uninitialized link group id"),
    );

    let plot_components = plot_wrapper.build_plot_components(total_plot_count, plot_graphics);

    fill_plots(
        ui,
        plot_components,
        axis_cfg,
        line_width,
        timer,
        is_reset_pressed,
        x_min_max,
        plot_name_filter,
    );
}

// Simply wraps some variables to make it harder to mess up a bunch of boolean arguments etc.
pub struct PlotWrapperHelper<'p> {
    plots: &'p mut Plots,
    display_percentage_plot: bool,
    display_to_hundred_plot: bool,
    display_thousands_plot: bool,
}

impl<'p> PlotWrapperHelper<'p> {
    pub fn new(plots: &'p mut Plots) -> Self {
        Self {
            plots,
            display_percentage_plot: false,
            display_to_hundred_plot: false,
            display_thousands_plot: false,
        }
    }

    pub fn should_display_percentage_plot(mut self, display_percentage_plot: bool) -> Self {
        self.display_percentage_plot = display_percentage_plot;
        self
    }
    pub fn should_display_to_hundred_plot(mut self, display_to_hundred_plot: bool) -> Self {
        self.display_to_hundred_plot = display_to_hundred_plot;
        self
    }
    pub fn should_display_thousands_plot(mut self, display_thousands_plot: bool) -> Self {
        self.display_thousands_plot = display_thousands_plot;
        self
    }

    pub fn build_plot_components(
        self,
        total_plot_count: u8,
        plot_graphics: [(Plot<'p>, PlotType); 3],
    ) -> Vec<(Plot<'_>, &PlotData, PlotType)> {
        let mut plot_components_list = Vec::with_capacity(total_plot_count.into());
        for (p_graphic, p_type) in plot_graphics {
            match p_type {
                PlotType::Percentage => {
                    if self.display_percentage_plot {
                        plot_components_list.push((p_graphic, self.plots.percentage(), p_type));
                    }
                }
                PlotType::Hundreds => {
                    if self.display_to_hundred_plot {
                        plot_components_list.push((p_graphic, self.plots.one_to_hundred(), p_type));
                    }
                }
                PlotType::Thousands => {
                    if self.display_thousands_plot {
                        plot_components_list.push((p_graphic, self.plots.thousands(), p_type));
                    }
                }
            }
        }
        plot_components_list
    }
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
) {
    let (plot_data, plot_type) = plot;
    plot_util::plot_lines(plot_ui, plot_data.plots(), name_filter, line_width);
    for plot_labels in plot_data.plot_labels() {
        for label in plot_labels.labels() {
            let point = PlotPoint::new(label.point()[0], label.point()[1]);
            let txt = RichText::new(label.text()).size(10.0);
            let txt = egui_plot::Text::new(point, txt);
            plot_ui.text(txt);
        }
    }
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
