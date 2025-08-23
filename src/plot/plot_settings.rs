use crate::plot::{
    axis_config::{AxisConfig, show_axis_settings},
    plot_settings::{
        log_groups::{LogGroupUIState, show_log_groups},
        series_plot_settings::SeriesPlotSettings,
    },
};
use date_settings::LoadedLogSettings;
use egui::{Color32, Frame, Key, Response, RichText};
use egui_phosphor::regular;
use mipmap_settings::MipMapSettings;
use plot_filter::{PlotNameFilter, PlotNameShow};
use plot_visibility_config::PlotVisibilityConfig;
use plotinator_plot_util::{MipMapConfiguration, PlotValues, Plots};
use plotinator_ui_util::theme_color;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

pub mod date_settings;
mod loaded_logs;
mod log_groups;
pub mod mipmap_settings;
mod plot_filter;
mod plot_visibility_config;
pub mod series_plot_settings;

#[derive(PartialEq, Deserialize, Serialize)]
struct PlotSettingsUi {
    show_loaded_logs: bool,
    show_filter_settings: bool,
    filter_settings_text: String,
}

impl Default for PlotSettingsUi {
    fn default() -> Self {
        Self {
            show_loaded_logs: Default::default(),
            show_filter_settings: Default::default(),
            filter_settings_text: format!("{} Filter", regular::FUNNEL),
        }
    }
}

impl PlotSettingsUi {
    pub fn filter_settings_text(&self) -> String {
        self.filter_settings_text.clone()
    }

    pub fn ui_toggle_show_filter(&mut self, ui: &mut egui::Ui) -> Response {
        let show_filter_text = self.filter_settings_text();
        ui.toggle_value(&mut self.show_filter_settings, show_filter_text)
    }
}

#[derive(Default, PartialEq, Deserialize, Serialize)]
pub struct PlotSettings {
    // The ID to assign to the next loaded log
    next_log_id: u16,
    /// Used for invalidating any cached values that determines plot layout etc.
    invalidate_plot: bool,
    visibility: PlotVisibilityConfig,
    display_percentage_plot: bool,
    display_hundreds_plot: bool,
    display_thousands_plot: bool,
    display_plot_count: u8,
    // Plot names and whether or not they should be shown (painted)
    plot_name_filter: PlotNameFilter,
    ps_ui: PlotSettingsUi,
    loaded_log_settings: Vec<LoadedLogSettings>,
    mipmap_settings: MipMapSettings,
    series_plot_settings: SeriesPlotSettings,
    #[serde(skip)]
    apply_deletions: bool,
    #[serde(skip)]
    log_group_ui_state: LogGroupUIState,
}

impl PlotSettings {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        axis_cfg: &mut AxisConfig,
        plots: &plotinator_plot_util::Plots,
    ) {
        if self.loaded_log_settings.is_empty() {
            ui.label(RichText::new("No Files Loaded").color(theme_color(
                ui,
                Color32::RED,
                Color32::DARK_RED,
            )));
            show_axis_settings(ui, axis_cfg);
            self.series_plot_settings.show(ui);
        } else {
            self.show_loaded_files(ui);
            self.ui_plot_filter_settings(ui, plots);
            self.mipmap_settings.show(ui);
            show_axis_settings(ui, axis_cfg);
            self.series_plot_settings.show(ui);
            self.visibility.toggle_visibility_ui(ui);
        }
    }

    fn ui_plot_filter_settings(&mut self, ui: &mut egui::Ui, plots: &plotinator_plot_util::Plots) {
        self.ps_ui.ui_toggle_show_filter(ui);
        if self.ps_ui.show_filter_settings {
            egui::Window::new(self.ps_ui.filter_settings_text())
                .open(&mut self.ps_ui.show_filter_settings)
                .show(ui.ctx(), |ui| {
                    self.plot_name_filter.show(ui, plots);
                });
            if ui.ctx().input(|i| i.key_pressed(Key::Escape)) {
                self.ps_ui.show_filter_settings = false;
            }
        }
    }

    fn ui_show_or_hide_all_buttons(ui: &mut egui::Ui, log_groups: &mut LogGroupUIState) {
        if ui
            .button(RichText::new("Hide all").strong())
            .on_hover_text("Hide all loaded logs")
            .clicked()
        {
            log_groups.hide_all();
        }
        ui.label("/");
        if ui
            .button(RichText::new("Show all").strong())
            .on_hover_text("Show all loaded logs")
            .clicked()
        {
            log_groups.show_all();
        }
        ui.separator();
        if ui
            .button(RichText::new("Collapse all").strong())
            .on_hover_text("Collapse all loaded logs")
            .clicked()
        {
            log_groups.collapse_all();
        }
        ui.label("/");
        if ui
            .button(RichText::new("Expand all").strong())
            .on_hover_text("Expand all loaded logs")
            .clicked()
        {
            log_groups.expand_all();
        }
    }

    /// Shows the loaded files window
    fn show_loaded_files(&mut self, ui: &mut egui::Ui) {
        let loaded_files_count = self.loaded_log_settings.len();
        let visibility_icon = if self.ps_ui.show_loaded_logs {
            regular::EYE
        } else {
            regular::EYE_SLASH
        };
        let show_loaded_logs_text = RichText::new(format!(
            "{visibility_icon} Loaded files ({loaded_files_count})",
        ));
        ui.toggle_value(
            &mut self.ps_ui.show_loaded_logs,
            show_loaded_logs_text.text(),
        );
        if self.ps_ui.show_loaded_logs {
            if ui.ctx().input(|i| i.key_pressed(Key::Escape))
                && !self.loaded_log_settings.iter().any(|s| s.clicked())
            {
                self.ps_ui.show_loaded_logs = false;
            }

            let frame = Frame::group(ui.style()).corner_radius(4.0);

            egui::Window::new(show_loaded_logs_text)
                .open(&mut self.ps_ui.show_loaded_logs)
                .show(ui.ctx(), |ui| {
                    frame.show(ui, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                Self::ui_show_or_hide_all_buttons(ui, &mut self.log_group_ui_state);
                            });
                            ui.add_space(2.0);
                            egui::Grid::new("log_settings_grid").show(ui, |ui| {
                                ui.label("");
                                ui.label("");
                                ui.label("");
                                let any_marked_for_deletion = self
                                    .loaded_log_settings
                                    .iter()
                                    .any(|s| s.marked_for_deletion());
                                let apply_text = if any_marked_for_deletion {
                                    RichText::new("Apply").strong()
                                } else {
                                    RichText::new("Apply")
                                };
                                if ui
                                    .add_enabled(
                                        any_marked_for_deletion,
                                        egui::Button::new(apply_text),
                                    )
                                    .clicked()
                                {
                                    self.apply_deletions = true;
                                }
                                ui.end_row();

                                show_log_groups(
                                    ui,
                                    &mut self.loaded_log_settings,
                                    &mut self.log_group_ui_state,
                                );
                            });
                        });
                    });
                });
        }
    }

    /// Needs to be called once (and only once!) per frame before querying for plot ui settings, such as
    /// how many plots to paint and more.
    pub fn refresh(&mut self, plots: &mut Plots) {
        #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
        puffin::profile_scope!("plot_settings.refresh");
        if self.apply_deletions {
            self.remove_if_marked_for_deletion(plots);
            self.apply_deletions = false;
        }
        self.set_highlighted(plots);
        self.update_plot_dates(plots);
        self.calc_plot_display_settings(plots);
        // If true then we set it to false such that it is only true for one frame
        if self.cached_plots_invalidated() {
            self.invalidate_plot = false;
        }
    }

    /// Whether or not to display the `percentage` plot area in the current frame
    pub fn display_percentage(&self) -> bool {
        self.display_percentage_plot
    }

    /// Whether or not to display the `one_to_hundred` plot area in the current frame
    pub fn display_hundreds(&self) -> bool {
        self.display_hundreds_plot
    }

    /// Whether or not to display the `thousands` plot area in the current frame
    pub fn display_thousands(&self) -> bool {
        self.display_thousands_plot
    }

    /// How many plots to paint in the current frame
    pub fn total_plot_count(&self) -> u8 {
        self.display_plot_count
    }

    /// Needs to be called once per frame before querying which plots to display
    pub fn calc_plot_display_settings(&mut self, plots: &Plots) {
        self.display_percentage_plot = self
            .visibility
            .should_display_percentage(plots.percentage().plots().is_empty());
        self.display_hundreds_plot = self
            .visibility
            .should_display_hundreds(plots.one_to_hundred().plots().is_empty());
        self.display_thousands_plot = self
            .visibility
            .should_display_thousands(plots.thousands().plots().is_empty());
        let mut total_plot_count: u8 = 0;
        total_plot_count += self.display_percentage_plot as u8;
        total_plot_count += self.display_hundreds_plot as u8;
        total_plot_count += self.display_thousands_plot as u8;
        self.display_plot_count = total_plot_count;
    }

    /// Adds a new plot name/label to the collection if it isn't already in the collection
    ///
    /// # Arguments
    /// - `plot_name` The name of the plot, i.e. the name that appears on the plot legend
    /// - `descriptive_name` the name of the logfile e.g. `Mbed Pid v6` or `frame-altimeter`
    pub fn add_plot_name_if_not_exists(&mut self, plot_name: &str, descriptive_name: &str) {
        if !self.plot_name_filter.contains(plot_name, descriptive_name) {
            self.plot_name_filter.add_plot(PlotNameShow::new(
                plot_name.to_owned(),
                true,
                descriptive_name.to_owned(),
            ));
        }
    }

    pub fn apply_filters<'pv>(
        &'pv self,
        plot_vals: &'pv [PlotValues],
    ) -> impl Iterator<Item = &'pv PlotValues> {
        #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
        puffin::profile_function!();
        let id_filter = self.log_id_filter();
        self.plot_name_filter
            .filter_plot_values(plot_vals, move |id| {
                for id_inst in &id_filter {
                    if *id_inst == id {
                        return false;
                    }
                }
                true
            })
    }

    /// Get the next ID for a loaded data format, used for when a new file is loaded and added to the collection of plot data and plot settings
    pub fn next_log_id(&mut self) -> u16 {
        self.next_log_id += 1;
        self.next_log_id
    }

    pub fn add_log_setting(&mut self, log_settings: LoadedLogSettings) {
        self.loaded_log_settings.push(log_settings);
        self.loaded_log_settings.sort_by(|a, b| {
            a.descriptive_name()
                .cmp(b.descriptive_name())
                .then_with(|| a.start_date().cmp(&b.start_date()))
        });
    }

    // The id filter specifies which plots belonging to which logs should not be painted on the plot ui.
    pub fn log_id_filter(&self) -> Vec<u16> {
        let mut log_id_filter: Vec<u16> = vec![];
        for settings in &self.loaded_log_settings {
            if !settings.show_log() {
                log_id_filter.push(settings.log_id());
            }
        }
        log_id_filter
    }

    fn update_plot_dates(&mut self, plots: &mut Plots) {
        for settings in &mut self.loaded_log_settings {
            date_settings::update_plot_dates(&mut self.invalidate_plot, plots, settings);
        }
    }

    fn set_highlighted(&self, plots: &mut Plots) {
        // Use a SmallVec as commonly there will be 0-1 ids to highlight, but with log grouping
        // there could potentially be many more
        let mut ids_to_highlight: SmallVec<[u16; 32]> = SmallVec::new();
        for log_setting in &self.loaded_log_settings {
            if log_setting.cursor_hovering_on() || log_setting.clicked() {
                ids_to_highlight.push(log_setting.log_id());
            }
        }
        let set_plot_highlight = |plot_data: &mut plotinator_plot_util::PlotData| {
            for pd in plot_data.plots_as_mut() {
                let should_highlight = ids_to_highlight.contains(&pd.log_id())
                    || self.plot_name_filter.should_highlight(pd.name());
                *pd.get_highlight_mut() = should_highlight;
            }
            for pl in plot_data.plot_labels_as_mut() {
                *pl.get_highlight_mut() = ids_to_highlight.contains(&pl.log_id());
            }
        };
        set_plot_highlight(plots.percentage_mut());
        set_plot_highlight(plots.one_to_hundred_mut());
        set_plot_highlight(plots.thousands_mut());

        // If hovering on any of the buttons where a plot area's visibility can be toggled, highlight all plots in that area.
        let set_all_plots_highlighted = |plot_data: &mut plotinator_plot_util::PlotData| {
            for pd in plot_data.plots_as_mut() {
                *pd.get_highlight_mut() = true;
            }
            for pl in plot_data.plot_labels_as_mut() {
                *pl.get_highlight_mut() = true;
            }
        };

        if self.visibility.hovered_display_percentage() {
            set_all_plots_highlighted(plots.percentage_mut());
        } else if self.visibility.hovered_display_to_hundreds() {
            set_all_plots_highlighted(plots.one_to_hundred_mut());
        } else if self.visibility.hovered_display_thousands() {
            set_all_plots_highlighted(plots.thousands_mut());
        }
    }

    // Remove log settings and plots that match their ID if they are marked for deletion
    fn remove_if_marked_for_deletion(&mut self, plots: &mut Plots) {
        // Get the log IDs for settings marked for deletion
        let log_ids_to_remove: Vec<u16> = self
            .loaded_log_settings
            .iter()
            .filter(|settings| settings.marked_for_deletion())
            .map(|settings| settings.log_id())
            .collect();

        // Return early if nothing to remove
        if log_ids_to_remove.is_empty() {
            return;
        }

        // Remove plots with matching log IDs from all plot types
        let remove_matching_plots = |plot_data: &mut plotinator_plot_util::PlotData| {
            // Remove from plot values
            plot_data
                .plots_as_mut()
                .retain(|plot| !log_ids_to_remove.contains(&plot.log_id()));

            // Remove from plot labels
            plot_data
                .plot_labels_as_mut()
                .retain(|label| !log_ids_to_remove.contains(&label.log_id()));
        };

        // Apply removal to all plot types
        remove_matching_plots(plots.percentage_mut());
        remove_matching_plots(plots.one_to_hundred_mut());
        remove_matching_plots(plots.thousands_mut());

        // Remove the settings marked for deletion
        self.loaded_log_settings
            .retain(|settings| !settings.marked_for_deletion());

        // Invalidate plot cache since we modified the data
        self.invalidate_plot = true;
    }

    /// Returns true if changes in plot settings occurred such that various cached values
    /// related to plot layout needs to be recalculated.
    pub fn cached_plots_invalidated(&self) -> bool {
        self.invalidate_plot
    }

    /// Returns the current `MipMap` settings as a [`MipMapConfiguration`]
    pub fn mipmap_cfg(&self) -> MipMapConfiguration {
        self.mipmap_settings.configuration()
    }

    /// Returns the current [`LinePlotSettings`]
    pub fn line_plot_settings(&self) -> SeriesPlotSettings {
        self.series_plot_settings
    }

    pub(crate) fn highlight(&self, ptype: super::PlotType) -> bool {
        match ptype {
            super::PlotType::Percentage => self.visibility.hovered_display_percentage(),
            super::PlotType::Hundreds => self.visibility.hovered_display_to_hundreds(),
            super::PlotType::Thousands => self.visibility.hovered_display_thousands(),
        }
    }
}
