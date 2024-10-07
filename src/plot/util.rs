use log_if::prelude::*;
use plot_util::{PlotValues, Plots, StoredPlotLabels};

use super::plot_settings::date_settings::LogStartDateSettings;

pub fn add_plot_data_to_plot_collections(
    log_start_date_settings: &mut Vec<LogStartDateSettings>,
    plots: &mut Plots,
    log: &dyn Plotable,
    plot_names_show: &mut Vec<(String, bool)>,
) {
    // This is how all logs get their log_id, and how each plot for each log gets their log_id
    let log_id = log_start_date_settings.len() + 1;

    log_start_date_settings.push(LogStartDateSettings::new(
        log_id,
        log.descriptive_name().to_owned(),
        log.first_timestamp(),
    ));
    for raw_plot in log.raw_plots() {
        match raw_plot.expected_range() {
            ExpectedPlotRange::Percentage => {
                add_plot_to_vector(plots.percentage_mut().plots_as_mut(), raw_plot, log_id);
            }
            ExpectedPlotRange::OneToOneHundred => {
                add_plot_to_vector(plots.one_to_hundred_mut().plots_as_mut(), raw_plot, log_id);
            }
            ExpectedPlotRange::Thousands => {
                add_plot_to_vector(plots.thousands_mut().plots_as_mut(), raw_plot, log_id);
            }
        }
        if !plot_names_show
            .iter()
            .any(|(name, _)| name == raw_plot.name())
        {
            plot_names_show.push((raw_plot.name().to_owned(), true));
        }
    }

    if let Some(plot_labels) = log.labels() {
        for labels in plot_labels {
            let owned_label_points = labels.label_points().to_owned();
            match labels.expected_range() {
                ExpectedPlotRange::Percentage => plots
                    .percentage_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, log_id)),
                ExpectedPlotRange::OneToOneHundred => {
                    plots
                        .one_to_hundred_mut()
                        .add_plot_labels(StoredPlotLabels::new(owned_label_points, log_id));
                }
                ExpectedPlotRange::Thousands => plots
                    .thousands_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, log_id)),
            }
        }
    }
}

/// Add plot to the list of plots if a plot with the same name isn't already in the vector
fn add_plot_to_vector(plots: &mut Vec<PlotValues>, raw_plot: &RawPlot, log_id: usize) {
    let plot_label = format!("#{id} {name}", id = log_id, name = raw_plot.name());
    if !plots.iter().any(|p| plot_label == p.label()) {
        plots.push(PlotValues::new(
            raw_plot.points().to_vec(),
            raw_plot.name().to_owned(),
            log_id,
        ));
    }
}
