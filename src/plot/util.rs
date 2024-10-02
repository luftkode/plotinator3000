use log_if::prelude::*;
use plot_util::{PlotWithName, Plots, StoredPlotLabels};

use super::date_settings::LogStartDateSettings;

pub fn calc_all_plot_x_min_max(plots: &Plots, x_min_max: &mut Option<(f64, f64)>) {
    calc_plot_x_min_max(plots.percentage().plots(), x_min_max);
    calc_plot_x_min_max(plots.percentage().plots(), x_min_max);
    calc_plot_x_min_max(plots.percentage().plots(), x_min_max);
}

// Go through each plot and find the minimum and maximum x-value (timestamp) and save it in `x_min_max`
fn calc_plot_x_min_max(plots: &[PlotWithName], x_min_max: &mut Option<(f64, f64)>) {
    for plot in plots {
        if plot.raw_plot.len() < 2 {
            continue;
        }
        let Some(first_x) = plot.raw_plot.first().and_then(|f| f.first()) else {
            continue;
        };
        let Some(last_x) = plot.raw_plot.last().and_then(|l| l.first()) else {
            continue;
        };
        if let Some((current_x_min, current_x_max)) = x_min_max {
            if first_x < current_x_min {
                *current_x_min = *first_x;
            }
            if last_x > current_x_max {
                *current_x_max = *last_x;
            }
        } else {
            x_min_max.replace((*first_x, *last_x));
        }
    }
}

pub fn add_plot_data_to_plot_collections(
    log_start_date_settings: &mut Vec<LogStartDateSettings>,
    plots: &mut Plots,
    log: &dyn Plotable,
) {
    let log_idx = log_start_date_settings.len() + 1;
    let log_id = format!("#{log_idx} {}", log.unique_name());

    log_start_date_settings.push(LogStartDateSettings::new(
        log_id.clone(),
        log.first_timestamp(),
    ));
    for raw_plot in log.raw_plots() {
        let plot_name = format!("{} #{}", raw_plot.name(), log_idx);
        match raw_plot.expected_range() {
            ExpectedPlotRange::Percentage => {
                add_plot_to_vector(
                    plots.percentage_mut().plots_as_mut(),
                    raw_plot,
                    &plot_name,
                    log_id.clone(),
                );
            }
            ExpectedPlotRange::OneToOneHundred => {
                add_plot_to_vector(
                    plots.one_to_hundred_mut().plots_as_mut(),
                    raw_plot,
                    &plot_name,
                    log_id.clone(),
                );
            }
            ExpectedPlotRange::Thousands => {
                add_plot_to_vector(
                    plots.thousands_mut().plots_as_mut(),
                    raw_plot,
                    &plot_name,
                    log_id.clone(),
                );
            }
        }
    }

    if let Some(plot_labels) = log.labels() {
        for labels in plot_labels {
            let owned_label_points = labels.label_points().to_owned();
            match labels.expected_range() {
                ExpectedPlotRange::Percentage => plots
                    .percentage_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, log_id.clone())),
                ExpectedPlotRange::OneToOneHundred => {
                    plots
                        .one_to_hundred_mut()
                        .add_plot_labels(StoredPlotLabels::new(owned_label_points, log_id.clone()));
                }
                ExpectedPlotRange::Thousands => plots
                    .thousands_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, log_id.clone())),
            }
        }
    }
}

/// Add plot to the list of plots if a plot with the same name isn't already in the vector
fn add_plot_to_vector(
    plots: &mut Vec<PlotWithName>,
    raw_plot: &RawPlot,
    plot_name: &str,
    log_id: String,
) {
    if !plots.iter().any(|p| p.name == *plot_name) {
        plots.push(PlotWithName::new(
            raw_plot.points().to_vec(),
            plot_name.to_owned(),
            log_id,
        ));
    }
}
