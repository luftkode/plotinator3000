use date_settings::LogStartDateSettings;
use log_if::plotable::Plotable;
use plot_ui::loaded_logs::LoadedLogsUi;
use plot_util::Plots;
use serde::{Deserialize, Serialize};

use crate::app::PlayBackButtonEvent;
use axis_config::AxisConfig;
use egui::{Id, Response};
use egui_plot::Legend;
use play_state::PlayState;
use plot_visibility_config::PlotVisibilityConfig;

mod axis_config;
mod date_settings;
mod play_state;
mod plot_graphics;
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
    show_filter_settings: bool,
    // Plot names and whether or not they should be shown (painted)
    plot_names_shown: Vec<(String, bool)>,
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
            show_filter_settings: false,
            plot_names_shown: vec![],
        }
    }
}

impl LogPlotUi {
    pub fn plot_count(&self) -> usize {
        self.plots.percentage().plots().len()
            + self.plots.one_to_hundred().plots().len()
            + self.plots.thousands().plots().len()
    }

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
            show_filter_settings,
            plot_names_shown,
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
            show_filter_settings,
            plot_names_shown,
        );

        if let Some(e) = playback_button_event {
            play_state.handle_playback_button_press(e);
        };
        let is_reset_pressed = matches!(playback_button_event, Some(PlayBackButtonEvent::Reset));

        let timer = play_state.time_since_update();

        for log in logs {
            util::add_plot_data_to_plot_collections(
                log_start_date_settings,
                plots,
                log.as_ref(),
                plot_names_shown,
            );
            log::info!("{plot_names_shown:?}");
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

        let plot_wrapper = plot_graphics::PlotWrapperHelper::new(plots)
            .should_display_percentage_plot(display_percentage_plot)
            .should_display_to_hundred_plot(display_to_hundred_plot)
            .should_display_thousands_plot(display_thousands_plot);
        let plot_name_filter: Vec<&str> = plot_names_shown
            .iter()
            .filter_map(|(name, show)| if *show { None } else { Some((*name).as_str()) })
            .collect();
        ui.vertical(|ui| {
            plot_graphics::paint_plots(
                ui,
                total_plot_count,
                legend_cfg,
                axis_config,
                *link_group,
                plot_wrapper,
                *line_width,
                timer,
                is_reset_pressed,
                *x_min_max,
                &plot_name_filter,
            );
        })
        .response
    }
}
