use chrono::{DateTime, NaiveDateTime, Utc};
use plot_util::{PlotWithName, StoredPlotLabels};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub struct LogStartDateSettings {
    pub log_id: String,
    pub original_start_date: DateTime<Utc>,
    pub start_date: DateTime<Utc>,
    pub clicked: bool,
    pub tmp_date_buf: String,
    pub err_msg: String,
    pub new_date_candidate: Option<NaiveDateTime>,
    pub date_changed: bool,
}

impl LogStartDateSettings {
    pub fn new(log_id: String, start_date: DateTime<Utc>) -> Self {
        Self {
            log_id,
            original_start_date: start_date,
            start_date,
            clicked: false,
            tmp_date_buf: String::new(),
            err_msg: String::new(),
            new_date_candidate: None,
            date_changed: false,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_plot_dates(
    invalidate_plot: &mut bool,
    percentage_plots: &mut [PlotWithName],
    percentage_plot_labels: &mut [StoredPlotLabels],
    to_hundreds_plots: &mut [PlotWithName],
    to_hundreds_plot_labels: &mut [StoredPlotLabels],
    to_thousands_plots: &mut [PlotWithName],
    to_thousands_plot_labels: &mut [StoredPlotLabels],
    settings: &mut LogStartDateSettings,
) {
    if settings.date_changed {
        apply_offset_to_plots(percentage_plots.iter_mut(), settings);
        apply_offset_to_plot_labels(percentage_plot_labels.iter_mut(), settings);
        apply_offset_to_plots(to_hundreds_plots.iter_mut(), settings);
        apply_offset_to_plot_labels(to_hundreds_plot_labels.iter_mut(), settings);
        apply_offset_to_plots(to_thousands_plots.iter_mut(), settings);
        apply_offset_to_plot_labels(to_thousands_plot_labels.iter_mut(), settings);

        settings.date_changed = false;
        *invalidate_plot = true;
    }
}

fn apply_offset_to_plots<'a, I>(plots: I, settings: &LogStartDateSettings)
where
    I: IntoIterator<Item = &'a mut PlotWithName>,
{
    for plot in plots {
        if plot.log_id == settings.log_id {
            offset_plot(plot, settings.start_date);
        }
    }
}

fn apply_offset_to_plot_labels<'a, I>(stored_plot_labels: I, settings: &LogStartDateSettings)
where
    I: IntoIterator<Item = &'a mut StoredPlotLabels>,
{
    for plot_label in stored_plot_labels {
        if plot_label.log_id() == settings.log_id {
            offset_plot_labels(plot_label, settings.start_date);
        }
    }
}

fn offset_plot(plot: &mut PlotWithName, new_start_date: DateTime<Utc>) {
    if let Some(first_point) = plot.raw_plot.first() {
        let first_point_date = first_point[0];
        let new_date_ns = new_start_date
            .timestamp_nanos_opt()
            .expect("Nanoseconds overflow") as f64;
        let offset = new_date_ns - first_point_date;

        log::debug!("Prev time: {first_point_date}, new: {new_date_ns}");
        log::debug!("Offsetting by: {offset}");

        for point in &mut plot.raw_plot {
            point[0] += offset;
        }
    }
}

fn offset_plot_labels(plot_labels: &mut StoredPlotLabels, new_start_date: DateTime<Utc>) {
    if let Some((first_point, _)) = plot_labels.label_points().first() {
        let first_point_date = first_point[0];
        let new_date_ns = new_start_date
            .timestamp_nanos_opt()
            .expect("Nanoseconds overflow") as f64;
        let offset = new_date_ns - first_point_date;
        for (point, _) in &mut plot_labels.label_points {
            point[0] += offset;
        }
    }
}
