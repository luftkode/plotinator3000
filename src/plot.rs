use std::time::Duration;

use egui_notify::Toasts;
use plot_settings::PlotSettings;
use plot_util::Plots;
use serde::{Deserialize, Serialize};

use axis_config::AxisConfig;
use egui::{Id, Response};
use egui_plot::{Legend, PlotPoints};

use crate::app::supported_formats::SupportedFormat;
mod axis_config;
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

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct ClickDelta {
    first_click: Option<(f64, f64)>,
    second_click: Option<(f64, f64)>,
}

impl ClickDelta {
    pub fn new() -> Self {
        Self {
            first_click: None,
            second_click: None,
        }
    }

    pub fn delta(&self) -> Option<(f64, f64)> {
        let (x1, y1) = self.first_click?;
        let (x2, y2) = self.second_click?;
        let delta_x = (x1 - x2).abs() / 1_000_000_000.0;
        let delta_y = (y1 - y2).abs();
        Some((delta_x, delta_y))
    }

    pub fn set_next_click(&mut self, click: (f64, f64)) {
        if self.second_click.is_some() {
            self.first_click = Some(click);
            self.second_click = None;
        } else if self.first_click.is_some() {
            self.second_click = Some(click);
        } else {
            self.first_click = Some(click);
        }
    }
}

#[allow(
    missing_debug_implementations,
    reason = "Legend is from egui_plot and doesn't implement debug"
)]
#[derive(PartialEq, Deserialize, Serialize)]
pub struct LogPlotUi {
    legend_cfg: Legend,
    line_width: f32,
    axis_config: AxisConfig,
    plots: Plots,
    plot_settings: PlotSettings,
    x_min_max: Option<(f64, f64)>,
    link_group: Option<Id>,
    click_delta: ClickDelta
}

impl Default for LogPlotUi {
    fn default() -> Self {
        Self {
            legend_cfg: Default::default(),
            line_width: 1.5,
            axis_config: Default::default(),
            plots: Plots::default(),
            plot_settings: PlotSettings::default(),
            x_min_max: None,
            link_group: None,
            click_delta: ClickDelta::new(),
        }
    }
}

impl LogPlotUi {
    pub fn plot_count(&self) -> usize {
        self.plots.percentage().plots().len()
            + self.plots.one_to_hundred().plots().len()
            + self.plots.thousands().plots().len()
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        loaded_files: &[SupportedFormat],
        toasts: &mut Toasts,
    ) -> Response {
        let Self {
            legend_cfg,
            line_width,
            axis_config,
            plots,
            plot_settings,
            x_min_max,
            link_group,
            click_delta,
        } = self;

        

        if link_group.is_none() {
            link_group.replace(ui.id().with("linked_plots"));
        }

        // Various stored knowledge about the plot needs to be reset and recalculated if the plot is invalidated
        if plot_settings.cached_plots_invalidated() {
            *x_min_max = None;
        }

        plots.calc_all_plot_x_min_max(x_min_max);

        plot_ui::show_settings_grid(ui, line_width, axis_config, plot_settings, click_delta);

        for log in loaded_files {
            util::add_plot_data_to_plot_collections(plots, log, plot_settings);
        }
        if !loaded_files.is_empty() {
            log::info!("Total data points: {}", plots.total_data_points());
            toasts
                .info(format!(
                    "Total data points in loaded files: {}",
                    plots.total_data_points(),
                ))
                .duration(Some(Duration::from_secs(20)));
        }

        plot_settings.refresh(plots);

        ui.vertical(|ui| {
            plot_graphics::paint_plots(
                ui,
                plots,
                plot_settings,
                legend_cfg,
                axis_config,
                link_group.expect("uninitialized link group id"),
                *line_width,
                click_delta
            );
        })
        .response
    }
}
