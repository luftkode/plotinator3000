use chrono::{DateTime, Datelike as _, Timelike as _, Utc};
use egui_plot::PlotBounds;
use plotinator_plot_util::{PlotData, Plots};
use plotinator_supported_formats::ParseInfo;
use plotinator_ui_util::date_editor::DateEditor;
use serde::{Deserialize, Serialize};

use crate::{loaded_log_metadata::LoadedLogMetadata, log_points_cutter::LogPointsCutter};

pub mod loaded_log_metadata;
pub mod log_points_cutter;

#[derive(PartialEq, Deserialize, Serialize)]
pub struct LoadedLogSettings {
    log_id: u16,
    log_descriptive_name: String,
    pub original_start_date: DateTime<Utc>,
    start_date: DateTime<Utc>,
    clicked: bool,
    pub start_date_editor: DateEditor,
    show: bool,
    log_metadata: Option<Vec<LoadedLogMetadata>>,
    parse_info: Option<ParseInfo>,
    marked_for_deletion: bool,
    is_hovered: bool,
    pub(crate) log_points_cutter: LogPointsCutter,
    start_date_changed: bool,
}

impl LoadedLogSettings {
    pub fn new(
        log_id: u16,
        descriptive_name: String,
        start_date: DateTime<Utc>,
        log_metadata: Option<Vec<(String, String)>>,
        parse_info: Option<ParseInfo>,
    ) -> Self {
        let log_metadata = log_metadata.map(|l| {
            l.into_iter()
                .map(|l| LoadedLogMetadata::new(l.0, l.1))
                .collect()
        });
        Self {
            log_id,
            log_descriptive_name: descriptive_name,
            original_start_date: start_date,
            start_date,
            clicked: false,
            log_points_cutter: LogPointsCutter::default(),
            start_date_editor: DateEditor::new(start_date),
            start_date_changed: false,
            show: true,
            log_metadata,
            parse_info,
            marked_for_deletion: false,
            is_hovered: false,
        }
    }

    /// Name of the log such as `Navsys` or `frame-magnetometer`
    pub fn descriptive_name(&self) -> &str {
        &self.log_descriptive_name
    }

    pub fn start_date(&self) -> DateTime<Utc> {
        self.start_date
    }

    pub fn starte_date_formatted(&self) -> String {
        let y = self.start_date.year();
        let m = self.start_date.month();
        let d = self.start_date.day();
        let hh = self.start_date.hour();
        let mm = self.start_date.minute();
        let ss = self.start_date.second();
        format!("{y}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}")
    }

    pub fn new_start_date(&mut self, new_start_date: DateTime<Utc>) {
        self.start_date = new_start_date;
        self.start_date_changed = true;
    }

    pub fn log_label(&self) -> String {
        format!(
            "{descriptive_name} #{log_id}",
            log_id = self.log_id,
            descriptive_name = self.log_descriptive_name,
        )
    }

    /// This is the ID that connects settings to plots
    pub fn log_id(&self) -> u16 {
        self.log_id
    }

    pub fn show_log(&self) -> bool {
        self.show
    }

    pub fn show_log_mut(&mut self) -> &mut bool {
        &mut self.show
    }

    pub fn log_metadata(&mut self) -> Option<&mut [LoadedLogMetadata]> {
        self.log_metadata.as_deref_mut()
    }

    pub fn parse_info(&self) -> Option<ParseInfo> {
        self.parse_info
    }

    pub fn marked_for_deletion(&self) -> bool {
        self.marked_for_deletion
    }

    pub fn mark_for_deletion(&mut self, mark: bool) {
        self.marked_for_deletion = mark;
    }

    pub fn marked_for_deletion_mut(&mut self) -> &mut bool {
        &mut self.marked_for_deletion
    }

    pub fn clicked(&self) -> bool {
        self.clicked
    }

    pub fn clicked_mut(&mut self) -> &mut bool {
        &mut self.clicked
    }

    pub fn toggle_clicked(&mut self) {
        self.clicked = !self.clicked;
    }

    pub fn cursor_hovering_on(&self) -> bool {
        self.is_hovered
    }

    pub fn cursor_hovering_on_mut(&mut self) -> &mut bool {
        &mut self.is_hovered
    }

    pub fn log_points_cutter_clicked(&self) -> bool {
        self.log_points_cutter.clicked()
    }

    pub fn set_log_points_cutter_clicked(&mut self, clicked: bool) {
        self.log_points_cutter.clicked = clicked;
    }

    pub fn show_log_points_cutter(
        &mut self,
        ui: &egui::Ui,
        log_name_date: &str,
        selected_box: Option<PlotBounds>,
    ) {
        self.log_points_cutter.show(ui, log_name_date, selected_box);
    }
}

pub fn update_plot_dates(
    invalidate_plot: &mut bool,
    plots: &mut Plots,
    settings: &mut LoadedLogSettings,
) {
    if let Some((start, end)) = settings.log_points_cutter.cut_points_x_range.take() {
        log::info!("Applying cut date range cut");
        let apply_cut_x_range = |plot_data: &mut PlotData| {
            for pd in plot_data.plots_as_mut() {
                if settings.log_id == pd.log_id() {
                    pd.cut_plot_within_x_range(start, end);
                }
            }
        };

        apply_cut_x_range(plots.percentage_mut());
        apply_cut_x_range(plots.one_to_hundred_mut());
        apply_cut_x_range(plots.thousands_mut());
        *invalidate_plot = true;
    }
    if let Some(cut) = settings.log_points_cutter.cut_points_outside_minmax.take() {
        log::info!("Applying cut outside min max range");
        let apply_cut_x_range = |plot_data: &mut PlotData| {
            for pd in plot_data.plots_as_mut() {
                if settings.log_id == pd.log_id() {
                    let (start, end) = cut.x_range;
                    let (min, max) = cut.y_min_max;
                    pd.cut_plot_outside_minmax(start, end, min, max);
                }
            }
        };

        apply_cut_x_range(plots.percentage_mut());
        apply_cut_x_range(plots.one_to_hundred_mut());
        apply_cut_x_range(plots.thousands_mut());
        *invalidate_plot = true;
    }

    if settings.start_date_changed {
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

        settings.start_date_changed = false;
        *invalidate_plot = true;
    }
}
