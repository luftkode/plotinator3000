use chrono::{DateTime, NaiveDateTime, Utc};
use plot_util::{PlotData, PlotWithName, Plots, StoredPlotLabels};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub struct LogStartDateSettings {
    log_id: usize,
    log_descriptive_name: String,
    pub original_start_date: DateTime<Utc>,
    start_date: DateTime<Utc>,
    pub clicked: bool,
    pub tmp_date_buf: String,
    pub err_msg: String,
    pub new_date_candidate: Option<NaiveDateTime>,
    pub date_changed: bool,
}

impl LogStartDateSettings {
    pub fn new(log_id: usize, descriptive_name: String, start_date: DateTime<Utc>) -> Self {
        Self {
            log_id,
            log_descriptive_name: descriptive_name,
            original_start_date: start_date,
            start_date,
            clicked: false,
            tmp_date_buf: String::new(),
            err_msg: String::new(),
            new_date_candidate: None,
            date_changed: false,
        }
    }

    pub fn start_date(&self) -> DateTime<Utc> {
        self.start_date
    }

    pub fn new_start_date(&mut self, new_start_date: DateTime<Utc>) {
        self.start_date = new_start_date;
    }

    pub fn log_label(&self) -> String {
        format!(
            "#{log_id} {descriptive_name} [{start_date}]",
            log_id = self.log_id,
            descriptive_name = self.log_descriptive_name,
            start_date = self.start_date.date_naive()
        )
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
                settings.log_id,
                settings.start_date,
                |p| p.log_id(),
                offset_plot,
            );
            apply_offset(
                plot_data.plot_labels_as_mut(),
                settings.log_id,
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

fn apply_offset<T, F>(
    items: &mut [T],
    log_id: usize,
    start_date: DateTime<Utc>,
    get_id: F,
    offset_fn: fn(&mut T, DateTime<Utc>),
) where
    F: Fn(&T) -> usize,
{
    for item in items {
        if get_id(item) == log_id {
            offset_fn(item, start_date);
        }
    }
}

fn offset_plot_labels(plot_labels: &mut StoredPlotLabels, new_start_date: DateTime<Utc>) {
    offset_data_iter(plot_labels.label_points_mut(), new_start_date);
}

fn offset_plot(plot: &mut PlotWithName, new_start_date: DateTime<Utc>) {
    offset_data_iter(plot.raw_plot.iter_mut(), new_start_date);
}

fn offset_data_iter<'i>(
    mut data_iter: impl Iterator<Item = &'i mut [f64; 2]>,
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
