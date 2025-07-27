use std::time::Duration;

use click_delta::ClickDelta;
#[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
use egui::Color32;
use egui_notify::Toasts;
use plot_settings::PlotSettings;
use plotinator_plot_util::{Plots, plots::MaxPlotBounds};
use plotinator_supported_formats::SupportedFormat;
use serde::{Deserialize, Serialize};

use axis_config::AxisConfig;
use egui::{Id, Response};
use egui_plot::Legend;

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
    MQTT(
        &'a [(plotinator_mqtt::MqttPlotPoints, Color32)],
        &'a mut bool,
    ),
}

#[derive(Debug, strum_macros::Display, Copy, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum PlotType {
    Percentage,
    Hundreds,
    Thousands,
}

#[derive(Default, Deserialize, Serialize)]
pub struct LogPlotUi {
    // We also store the raw files so they are easy to export
    stored_plot_files: Vec<SupportedFormat>,
    legend_cfg: Legend,
    axis_config: AxisConfig,
    plots: Plots,
    plot_settings: PlotSettings,
    max_bounds: MaxPlotBounds, // The maximum bounds for the plot, used for resetting zoom
    link_group: Option<Id>,
    click_delta: ClickDelta,
}

impl LogPlotUi {
    pub fn stored_plot_files(&self) -> &[SupportedFormat] {
        &self.stored_plot_files
    }

    pub fn plot_count(&self) -> usize {
        self.plots.percentage().plots().len()
            + self.plots.one_to_hundred().plots().len()
            + self.plots.thousands().plots().len()
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        first_frame: &mut bool,
        loaded_files: &[SupportedFormat],
        toasts: &mut Toasts,
        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))] mqtt: &mut crate::mqtt::Mqtt,
    ) -> Response {
        #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
        puffin::profile_scope!("Plot_UI");

        let Self {
            legend_cfg,
            axis_config,
            plots,
            plot_settings,
            max_bounds,
            link_group,
            click_delta,
            stored_plot_files,
        } = self;

        if link_group.is_none() {
            link_group.replace(ui.id().with("linked_plots"));
        }

        plot_ui::show_settings_grid(ui, axis_config, plot_settings);

        for log in loaded_files {
            util::add_plot_data_to_plot_collections(plots, log, plot_settings);
            stored_plot_files.push(log.clone());
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
        if *first_frame {
            plots.build_plots();
            *first_frame = false;
        } else if plot_settings.cached_plots_invalidated() {
            plots.build_plots();
            *max_bounds = MaxPlotBounds::default();
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
            let mqtt_plots = crate::mqtt::Mqtt::plots(mqtt.mqtt_plot_data.as_ref());
            if mqtt_plots.is_empty() {
                PlotMode::Logs(plots)
            } else {
                PlotMode::MQTT(mqtt_plots, &mut mqtt.set_auto_bounds)
            }
        };
        #[cfg(not(all(not(target_arch = "wasm32"), feature = "mqtt")))]
        let mode = PlotMode::Logs(plots);

        ui.vertical(|ui| {
            plot_graphics::paint_plots(
                ui,
                reset_plot_bounds,
                plot_settings,
                legend_cfg,
                axis_config,
                link_group.expect("uninitialized link group id"),
                click_delta,
                mode,
            );
        })
        .response
    }
}
