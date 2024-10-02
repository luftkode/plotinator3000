use chrono::{DateTime, NaiveDateTime, Utc};
use plot_util::{PlotData, PlotWithName, Plots, StoredPlotLabels};
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

fn apply_offset<T, F>(
    items: &mut [T],
    log_id: &str,
    start_date: DateTime<Utc>,
    get_id: F,
    offset_fn: fn(&mut T, DateTime<Utc>),
) where
    F: Fn(&T) -> &str,
{
    for item in items {
        if get_id(item) == log_id {
            offset_fn(item, start_date);
        }
    }
}

pub fn update_plot_dates(
    invalidate_plot: &mut bool,
    plots: &mut Plots,
    settings: &mut LogStartDateSettings,
) {
    if settings.date_changed {
        let apply_offsets = |plot_data: &mut PlotData| {
            apply_offset(
                plot_data.plots_as_mut(),
                &settings.log_id,
                settings.start_date,
                |p| &p.log_id,
                offset_plot,
            );
            apply_offset(
                plot_data.plot_labels_as_mut(),
                &settings.log_id,
                settings.start_date,
                StoredPlotLabels::log_id,
                offset_plot_labels,
            );
        };

        apply_offsets(plots.percentage_mut());
        apply_offsets(plots.one_to_hundred_mut());
        apply_offsets(plots.thousands_mut());

        settings.date_changed = false;
        *invalidate_plot = true;
    }
}

fn offset_plot_labels(plot_labels: &mut StoredPlotLabels, new_start_date: DateTime<Utc>) {
    offset_data_iter(plot_labels.label_points_mut(), new_start_date);
}

fn offset_plot(plot: &mut PlotWithName, new_start_date: DateTime<Utc>) {
    offset_data(&mut plot.raw_plot, new_start_date);
}

fn offset_data(data: &mut [[f64; 2]], new_start_date: DateTime<Utc>) {
    if let Some(first_point) = data.first() {
        let new_date_ns = new_start_date
            .timestamp_nanos_opt()
            .expect("Nanoseconds overflow") as f64;
        let offset = new_date_ns - first_point[0];
        for point in data.iter_mut() {
            point[0] += offset;
        }
    }
}

fn offset_data_iter<'a>(
    mut data_iter: impl Iterator<Item = &'a mut [f64; 2]>,
    new_start_date: DateTime<Utc>,
) {
    if let Some(first_point) = data_iter.next() {
        let new_date_ns = new_start_date
            .timestamp_nanos_opt()
            .expect("Nanoseconds overflow") as f64;
        let offset = new_date_ns - first_point[0];
        for point in data_iter {
            point[0] += offset;
        }
    }
}
