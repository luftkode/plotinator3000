use date_settings::LogStartDateSettings;
use log_if::plotable::Plotable;
use plot_ui::loaded_logs::LoadedLogsUi;
use plot_util::{PlotData, Plots};
use serde::{Deserialize, Serialize};

use crate::app::PlayBackButtonEvent;
use axis_config::AxisConfig;
use egui::{Id, Response, RichText, Ui};
use egui_plot::{AxisHints, HPlacement, Legend, Plot, PlotPoint};
use play_state::{playback_update_plot, PlayState};
use plot_visibility_config::PlotVisibilityConfig;

mod axis_config;
mod date_settings;
mod play_state;
mod plot_ui;
mod plot_visibility_config;
mod util;

#[derive(Debug, strum_macros::Display, Copy, Clone, PartialEq, Eq)]
pub enum PlotType {
    Percentage,
    Hundreds,
    Thousands,
}

#[allow(missing_debug_implementations)] // Legend is from egui_plot and doesn't implement debug
#[derive(PartialEq, Deserialize, Serialize)]
pub struct LogPlotUi {
    legend_cfg: Legend,
    line_width: f32,
    axis_config: AxisConfig,
    play_state: PlayState,
    plots: Plots,
    plot_visibility: PlotVisibilityConfig,
    log_start_date_settings: Vec<LogStartDateSettings>,
    x_min_max: Option<(f64, f64)>,
    // Various info about the plot is invalidated if this is true (so it needs to be recalculated)
    invalidate_plot: bool,
    link_group: Option<Id>,
    show_loaded_logs: bool,
}

impl Default for LogPlotUi {
    fn default() -> Self {
        Self {
            legend_cfg: Default::default(),
            line_width: 1.5,
            axis_config: Default::default(),
            play_state: PlayState::default(),
            plots: Plots::default(),
            plot_visibility: PlotVisibilityConfig::default(),
            log_start_date_settings: vec![],
            x_min_max: None,
            invalidate_plot: false,
            link_group: None,
            show_loaded_logs: false,
        }
    }
}

impl LogPlotUi {
    pub fn is_playing(&self) -> bool {
        self.play_state.is_playing()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, logs: &[Box<dyn Plotable>]) -> Response {
        let Self {
            legend_cfg,
            line_width,
            axis_config,
            play_state,
            plots,
            plot_visibility,
            log_start_date_settings,
            x_min_max,
            invalidate_plot,
            link_group,
            show_loaded_logs,
        } = self;
        if link_group.is_none() {
            link_group.replace(ui.id().with("linked_plots"));
        }

        // Various stored knowledge about the plot needs to be reset and recalculated if the plot is invalidated
        if *invalidate_plot {
            *x_min_max = None;
            *invalidate_plot = false;
        }

        plots.calc_all_plot_x_min_max(x_min_max);

        let mut playback_button_event = None;

        plot_ui::show_settings_grid(
            ui,
            play_state,
            &mut playback_button_event,
            line_width,
            axis_config,
            plot_visibility,
            LoadedLogsUi::state(log_start_date_settings, show_loaded_logs),
        );

        if let Some(e) = playback_button_event {
            play_state.handle_playback_button_press(e);
        };
        let is_reset_pressed = matches!(playback_button_event, Some(PlayBackButtonEvent::Reset));
        let timer = play_state.time_since_update();

        for log in logs {
            util::add_plot_data_to_plot_collections(log_start_date_settings, plots, log.as_ref());
        }
        for settings in log_start_date_settings {
            date_settings::update_plot_dates(invalidate_plot, plots, settings);
        }

        // Calculate the number of plots to display
        let mut total_plot_count: u8 = 0;
        let display_percentage_plot =
            plot_visibility.should_display_percentage(plots.percentage().plots().is_empty());
        total_plot_count += display_percentage_plot as u8;
        let display_to_hundred_plot =
            plot_visibility.should_display_hundreds(plots.one_to_hundred().plots().is_empty());
        total_plot_count += display_to_hundred_plot as u8;
        let display_thousands_plot =
            plot_visibility.should_display_thousands(plots.thousands().plots().is_empty());
        total_plot_count += display_thousands_plot as u8;

        ui.vertical(|ui| {
            let plot_height = ui.available_height() / (total_plot_count as f32);

            let (percentage_plot, to_hundred, thousands) = build_all_plot_uis(
                plot_height,
                legend_cfg,
                axis_config,
                link_group.expect("uninitialized link group id"),
            );

            let mut plot_components_list = Vec::with_capacity(total_plot_count.into());
            if display_percentage_plot {
                plot_components_list.push((
                    percentage_plot,
                    plots.percentage(),
                    PlotType::Percentage,
                ));
            }
            if display_to_hundred_plot {
                ui.separator();
                plot_components_list.push((to_hundred, plots.one_to_hundred(), PlotType::Hundreds));
            }

            if display_thousands_plot {
                ui.separator();
                plot_components_list.push((thousands, plots.thousands(), PlotType::Thousands));
            }

            fill_plots(
                ui,
                plot_components_list,
                axis_config,
                *line_width,
                timer,
                is_reset_pressed,
                *x_min_max,
            );
        })
        .response
    }
}

/// Iterate and fill/paint all plots with plot data
fn fill_plots(
    gui: &mut Ui,
    plot_components: Vec<(Plot<'_>, &PlotData, PlotType)>,
    axis_config: &mut AxisConfig,
    line_width: f32,
    timer: Option<f64>,
    is_reset_pressed: bool,
    x_min_max: Option<(f64, f64)>,
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
            );
        });
    }
}

/// Iterate and fill/paint a plot with plot data
fn fill_plot(
    plot_ui: &mut egui_plot::PlotUi,
    plot: (&PlotData, PlotType),
    axis_config: &mut AxisConfig,
    line_width: f32,
    timer: Option<f64>,
    is_reset_pressed: bool,
    x_min_max: Option<(f64, f64)>,
) {
    let (plot_data, plot_type) = plot;
    plot_util::plot_lines(plot_ui, plot_data.plots(), line_width);
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
fn build_all_plot_uis<'a>(
    plot_height: f32,
    legend_cfg: &Legend,
    axis_config: &AxisConfig,
    link_group: Id,
) -> (Plot<'a>, Plot<'a>, Plot<'a>) {
    let x_axes = vec![AxisHints::new_x()
        .label("Time")
        .formatter(crate::util::format_time)];

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
    (percentage_plot, to_hundred, thousands)
}

fn build_plot_ui<'a>(
    name: &str,
    plot_height: f32,
    legend_cfg: Legend,
    axis_config: &AxisConfig,
    x_axes: Vec<AxisHints<'a>>,
    link_group: Id,
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
