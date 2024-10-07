use std::time::Duration;

use egui_notify::Toasts;
use log_if::plotable::Plotable;
use plot_settings::PlotSettings;
use plot_util::Plots;
use serde::{Deserialize, Serialize};

use crate::app::PlayBackButtonEvent;
use axis_config::AxisConfig;
use egui::{Id, Response};
use egui_plot::Legend;
use play_state::PlayState;

mod axis_config;
mod play_state;
mod plot_graphics;
mod plot_settings;
mod plot_ui;
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
    plot_settings: PlotSettings,
    x_min_max: Option<(f64, f64)>,
    link_group: Option<Id>,
}

impl Default for LogPlotUi {
    fn default() -> Self {
        Self {
            legend_cfg: Default::default(),
            line_width: 1.5,
            axis_config: Default::default(),
            play_state: PlayState::default(),
            plots: Plots::default(),
            plot_settings: PlotSettings::default(),
            x_min_max: None,
            link_group: None,
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

    #[allow(clippy::too_many_lines)]
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        logs: &[Box<dyn Plotable>],
        toasts: &mut Toasts,
    ) -> Response {
        let Self {
            legend_cfg,
            line_width,
            axis_config,
            play_state,
            plots,
            plot_settings,
            x_min_max,
            link_group,
        } = self;
        if link_group.is_none() {
            link_group.replace(ui.id().with("linked_plots"));
        }

        // Various stored knowledge about the plot needs to be reset and recalculated if the plot is invalidated
        if plot_settings.cached_plots_invalidated() {
            *x_min_max = None;
        }

        plots.calc_all_plot_x_min_max(x_min_max);

        let mut playback_button_event = None;

        plot_ui::show_settings_grid(
            ui,
            play_state,
            &mut playback_button_event,
            line_width,
            axis_config,
            plot_settings,
        );

        if let Some(e) = playback_button_event {
            play_state.handle_playback_button_press(e);
        };
        let is_reset_pressed = matches!(playback_button_event, Some(PlayBackButtonEvent::Reset));
        let timer = play_state.time_since_update();

        if !logs.is_empty() {
            let mut log_names_str = String::new();
            for l in logs {
                log_names_str.push('\n');
                log_names_str.push('\t');
                log_names_str.push_str(l.descriptive_name());
            }
            toasts
                .info(format!(
                    "{} log{} added{log_names_str}",
                    logs.len(),
                    if logs.len() == 1 { "" } else { "s" }
                ))
                .duration(Some(Duration::from_secs(2)));
        }
        for log in logs {
            util::add_plot_data_to_plot_collections(plots, log.as_ref(), plot_settings);
        }

        plot_settings.refresh(plots);

        ui.vertical(|ui| {
            plot_graphics::paint_plots(
                ui,
                plots,
                plot_settings,
                legend_cfg,
                axis_config,
                *link_group,
                *line_width,
                timer,
                is_reset_pressed,
                *x_min_max,
            );
        })
        .response
    }
}
