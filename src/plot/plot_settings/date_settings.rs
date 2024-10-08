use chrono::{DateTime, NaiveDateTime, Utc};
use plot_util::{PlotData, Plots};
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
    show: bool,
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
            show: true,
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
            start_date = self.start_date.naive_utc()
        )
    }

    pub fn log_id(&self) -> usize {
        self.log_id
    }

    pub fn show_log(&self) -> bool {
        self.show
    }

    pub fn show_log_mut(&mut self) -> &mut bool {
        &mut self.show
    }
}

pub fn update_plot_dates(
    invalidate_plot: &mut bool,
    plots: &mut Plots,
    settings: &mut LogStartDateSettings,
) {
    if settings.date_changed {
        let apply_offsets = |plot_data: &mut PlotData| {
            for pd in plot_data.plots_as_mut() {
                if settings.log_id == pd.log_id() {
                    pd.offset_plot(settings.start_date());
                }
            }

            for pl in plot_data.plot_labels_as_mut() {
                if settings.log_id == pl.log_id() {
                    pl.offset_labels(settings.start_date());
                }
            }
        };

        apply_offsets(plots.percentage_mut());
        apply_offsets(plots.one_to_hundred_mut());
        apply_offsets(plots.thousands_mut());

        settings.date_changed = false;
        *invalidate_plot = true;
    }
}
