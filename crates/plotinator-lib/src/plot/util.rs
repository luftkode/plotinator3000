use log_if::prelude::*;
use plot_util::{Plots, StoredPlotLabels};

use crate::app::supported_formats::SupportedFormat;

use super::plot_settings::{date_settings::LoadedLogSettings, PlotSettings};

pub fn add_plot_data_to_plot_collections(
    plots: &mut Plots,
    data: &SupportedFormat,
    plot_settings: &mut PlotSettings,
) {
    // This is how all logs get their log_id, and how each plot for each log gets their log_id
    let data_id = plot_settings.next_log_id();

    plot_settings.add_log_setting(LoadedLogSettings::new(
        data_id,
        data.descriptive_name().to_owned(),
        data.first_timestamp(),
        data.metadata(),
        data.parse_info(),
    ));
    for raw_plot in data.raw_plots() {
        match raw_plot.expected_range() {
            ExpectedPlotRange::Percentage => {
                plots
                    .percentage_mut()
                    .add_plot_if_not_exists(raw_plot, data_id);
            }
            ExpectedPlotRange::OneToOneHundred => {
                plots
                    .one_to_hundred_mut()
                    .add_plot_if_not_exists(raw_plot, data_id);
            }
            ExpectedPlotRange::Thousands => {
                plots
                    .thousands_mut()
                    .add_plot_if_not_exists(raw_plot, data_id);
            }
        }
        plot_settings.add_plot_name_if_not_exists(raw_plot.name());
    }

    if let Some(plot_labels) = data.labels() {
        for labels in plot_labels {
            let owned_label_points = labels.label_points().to_owned();
            match labels.expected_range() {
                ExpectedPlotRange::Percentage => plots
                    .percentage_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, data_id)),
                ExpectedPlotRange::OneToOneHundred => {
                    plots
                        .one_to_hundred_mut()
                        .add_plot_labels(StoredPlotLabels::new(owned_label_points, data_id));
                }
                ExpectedPlotRange::Thousands => plots
                    .thousands_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, data_id)),
            }
        }
    }
}
