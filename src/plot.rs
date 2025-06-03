use std::time::Duration;

use click_delta::ClickDelta;
use egui_notify::Toasts;
use plot_settings::PlotSettings;
use plotinator_plot_util::{Plots, plots::MaxPlotBounds};
use serde::{Deserialize, Serialize};

use axis_config::AxisConfig;
use egui::{Id, Response};
use egui_plot::Legend;

use crate::app::supported_formats::SupportedFormat;
mod axis_config;
mod click_delta;
mod plot_graphics;
#[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
pub mod plot_mqtt;
mod plot_settings;
mod plot_ui;
mod util;
mod x_axis_formatter;

pub enum PlotMode<'a> {
    Logs(&'a mut Plots),
    #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
    MQTT(&'a [plotinator_mqtt::MqttPlotPoints], &'a mut bool),
}

#[derive(Debug, strum_macros::Display, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum PlotType {
    Percentage,
    Hundreds,
    Thousands,
}

#[derive(Deserialize, Serialize)]
pub struct LogPlotUi {
    #[serde(skip)]
    init: bool,
    legend_cfg: Legend,
    line_width: f32,
    axis_config: AxisConfig,
    plots: Plots,
    plot_settings: PlotSettings,
    max_bounds: MaxPlotBounds, // The maximum bounds for the plot, used for resetting zoom
    link_group: Option<Id>,
    click_delta: ClickDelta,
}

impl Default for LogPlotUi {
    fn default() -> Self {
        Self {
            init: false,
            legend_cfg: Default::default(),
            line_width: 1.5,
            axis_config: Default::default(),
            plots: Plots::default(),
            plot_settings: PlotSettings::default(),
            max_bounds: MaxPlotBounds::default(),
            link_group: None,
            click_delta: ClickDelta::default(),
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
        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))] mqtt: &mut crate::mqtt::Mqtt,
    ) -> Response {
        #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
        puffin::profile_scope!("Plot_UI");

        let Self {
            init,
            legend_cfg,
            line_width,
            axis_config,
            plots,
            plot_settings,
            max_bounds,
            link_group,
            click_delta,
        } = self;

        if link_group.is_none() {
            link_group.replace(ui.id().with("linked_plots"));
        }

        plot_ui::show_settings_grid(ui, line_width, axis_config, plot_settings);

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

        let mut reset_plot_bounds = false;
        // Various stored knowledge about the plot needs to be reset and recalculated if the plot is invalidated
        if plot_settings.cached_plots_invalidated() || !*init {
            *max_bounds = MaxPlotBounds::default();
            plots.build_plots();
            plots.calc_all_plot_max_bounds(max_bounds);
            reset_plot_bounds = true;
        } else if !loaded_files.is_empty() {
            plots.calc_all_plot_max_bounds(max_bounds);
            reset_plot_bounds = true;
        }

        plot_settings.refresh(plots);

        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
        let mode = {
            mqtt.show_waiting_for_initial_data(ui);
            let mqtt_plots = crate::mqtt::Mqtt::plots(mqtt.mqtt_data_receiver.as_ref());
            if mqtt_plots.is_empty() {
                PlotMode::Logs(plots)
            } else {
                PlotMode::MQTT(mqtt_plots, &mut mqtt.set_auto_bounds)
            }
        };
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "mqtt")))]
        let mode = PlotMode::Logs(plots);

        let response = ui
            .vertical(|ui| {
                plot_graphics::paint_plots(
                    ui,
                    reset_plot_bounds,
                    plot_settings,
                    legend_cfg,
                    axis_config,
                    link_group.expect("uninitialized link group id"),
                    *line_width,
                    click_delta,
                    mode,
                );
            })
            .response;
        *init = true;
        response
    }
}
