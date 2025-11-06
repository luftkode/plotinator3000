use std::time::Duration;

use click_delta::ClickDelta;
#[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
use egui::Color32;
use egui_notify::Toasts;
use plot_settings::PlotSettings;
use plotinator_plot_util::{CookedPlot, Plots, plots::MaxPlotBounds};
use plotinator_supported_formats::SupportedFormat;
use plotinator_ui_util::{box_selection::BoxSelection, format_large_number};
use serde::{Deserialize, Serialize};

use plotinator_background_parser::loaded_format::LoadedSupportedFormat;

use axis_config::AxisConfig;
use egui::{Id, Response};
use egui_plot::Legend;
use smallvec::SmallVec;

mod axis_config;
mod click_delta;
mod plot_graphics;
#[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
pub mod plot_mqtt;
pub mod plot_settings;
mod plot_ui;
mod util;
mod x_axis_formatter;

/// if a log is loaded from content that exceeds this many unparsed bytes:
/// - Show a toasts warning notification
/// - Show warnings in the UI when viewing parse info for the loaded log
pub const WARN_ON_UNPARSED_BYTES_THRESHOLD: usize = 128;

pub enum PlotMode<'a> {
    Logs(&'a mut Plots),
    #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
    MQTT {
        plots: &'a [(plotinator_mqtt_ui::plot::MqttPlotPoints, Color32)],
        auto_bounds: &'a mut bool,
        plot_scroller: &'a mut plotinator_mqtt_ui::connection::PlotScroller,
    },
}

#[derive(Default, Deserialize, Serialize)]
pub struct LogPlotUi {
    // We also store the raw files so they are easy to export
    #[serde(skip)]
    stored_plot_files: Vec<SupportedFormat>,
    legend_cfg: Legend,
    axis_config: AxisConfig,
    #[serde(skip)]
    plots: Plots,
    plot_settings: PlotSettings,
    #[serde(skip)]
    max_bounds: MaxPlotBounds, // The maximum bounds for the plot, used for resetting zoom
    link_group: Option<Id>,
    click_delta: ClickDelta,
    #[serde(skip)]
    total_data_points: u32,
    #[serde(skip)]
    box_selection: BoxSelection,
}

impl LogPlotUi {
    pub fn stored_plot_files(&self) -> &[SupportedFormat] {
        &self.stored_plot_files
    }

    pub fn individual_plots(&self) -> impl Iterator<Item = &CookedPlot> {
        self.plots.individual_plots()
    }

    pub fn plot_count(&self) -> usize {
        self.plots.percentage().plots().len()
            + self.plots.one_to_hundred().plots().len()
            + self.plots.thousands().plots().len()
    }

    pub fn total_data_points(&self) -> u32 {
        self.total_data_points
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        first_frame: &mut bool,
        loaded_formats: &mut SmallVec<[LoadedSupportedFormat; 1]>,
        toasts: &mut Toasts,
        #[cfg(all(not(target_arch = "wasm32"), feature = "map"))]
        map_cmd: &mut plotinator_map_ui::commander::MapUiCommander,
        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
        mqtt: &mut plotinator_mqtt_ui::connection::MqttConnection,
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
            box_selection,
            stored_plot_files,
            total_data_points,
        } = self;

        if link_group.is_none() {
            link_group.replace(ui.id().with("linked_plots"));
        }

        let mut reset_plot_bounds = false;

        plot_ui::show_settings_grid(
            ui,
            axis_config,
            plot_settings,
            plots,
            box_selection.selected(),
            &mut reset_plot_bounds,
        );

        for format in loaded_formats.iter_mut() {
            plot_settings.add_log_setting(format.take_settings());
            let cooked_plots = format.take_cooked_plots();
            for plot in &cooked_plots {
                plot_settings.add_plot_name_if_not_exists(
                    plot.ty().clone(),
                    plot.associated_descriptive_name(),
                    plot.log_id(),
                );
            }
            plots.add_plots(cooked_plots);
            plots.add_plot_labels(format.take_cooked_labels());
            stored_plot_files.push(format.take_supported_format());
        }

        if !loaded_formats.is_empty() {
            *total_data_points = plots.total_data_points() as u32;
            log::info!(
                "Total data points: {}",
                format_large_number(*total_data_points)
            );
            toasts
                .info(format!(
                    "Total data points in loaded files: {}",
                    format_large_number(*total_data_points),
                ))
                .duration(Some(Duration::from_secs(10)));
        }

        // Various stored knowledge about the plot needs to be reset and recalculated if the plot is invalidated
        if *first_frame {
            plots.build_plots();
            *first_frame = false;
        } else if plot_settings.cached_plots_invalidated() {
            plots.build_plots();
            *max_bounds = MaxPlotBounds::default();
            plots.calc_all_plot_max_bounds(max_bounds);
            reset_plot_bounds = true;
        } else if !loaded_formats.is_empty() {
            plots.calc_all_plot_max_bounds(max_bounds);
            reset_plot_bounds = true;
        }

        plot_settings.refresh(plots);

        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
        let mode = {
            mqtt.show_waiting_for_initial_data(ui);
            let mqtt_plots =
                plotinator_mqtt_ui::connection::MqttConnection::plots(mqtt.mqtt_plot_data.as_ref());
            if mqtt_plots.is_empty() {
                PlotMode::Logs(plots)
            } else {
                PlotMode::MQTT {
                    plots: mqtt_plots,
                    auto_bounds: &mut mqtt.set_auto_bounds,
                    plot_scroller: &mut mqtt.plot_scroller,
                }
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
                box_selection,
                #[cfg(all(not(target_arch = "wasm32"), feature = "map"))]
                map_cmd,
                mode,
            );
        })
        .response
    }
}
