use log_if::prelude::*;
use plot_util::PlotWithName;

use super::date_settings::LogStartDateSettings;

pub fn calc_all_plot_x_min_max(
    percentage: &[PlotWithName],
    to_hundreds: &[PlotWithName],
    to_thousands: &[PlotWithName],
    x_min_max: &mut Option<(f64, f64)>,
) {
    calc_plot_x_min_max(percentage, x_min_max);
    calc_plot_x_min_max(to_hundreds, x_min_max);
    calc_plot_x_min_max(to_thousands, x_min_max);
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
    percentage_plots: &mut Vec<PlotWithName>,
    to_hundreds_plots: &mut Vec<PlotWithName>,
    to_thousands_plots: &mut Vec<PlotWithName>,
    log: &dyn Plotable,
    idx: usize,
) {
    let log_id = format!("#{} {}", idx + 1, log.unique_name());
    if !log_start_date_settings
        .iter()
        .any(|settings| *settings.log_id == log_id)
    {
        log_start_date_settings.push(LogStartDateSettings::new(
            log_id.clone(),
            log.first_timestamp(),
        ));
    }

    for raw_plot in log.raw_plots() {
        let plot_name = format!("{} #{}", raw_plot.name(), idx + 1);
        match raw_plot.expected_range() {
            ExpectedPlotRange::Percentage => {
                add_plot_to_vector(percentage_plots, raw_plot, &plot_name, log_id.clone());
            }
            ExpectedPlotRange::OneToOneHundred => {
                add_plot_to_vector(to_hundreds_plots, raw_plot, &plot_name, log_id.clone());
            }
            ExpectedPlotRange::Thousands => {
                add_plot_to_vector(to_thousands_plots, raw_plot, &plot_name, log_id.clone());
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
