use date_settings::LogStartDateSettings;
use egui::{Key, Response, RichText};
use egui_phosphor::regular;
use plot_util::Plots;
use plot_visibility_config::PlotVisibilityConfig;
use serde::{Deserialize, Serialize};

pub mod date_settings;
mod loaded_logs;
mod plot_visibility_config;

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
            filter_settings_text: format!("{} Filter {}", regular::FUNNEL, regular::CHART_LINE),
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
    /// Used for invalidating any cached values that determines plot layout etc.
    invalidate_plot: bool,
    visibility: PlotVisibilityConfig,
    display_percentage_plot: bool,
    display_hundreds_plot: bool,
    display_thousands_plot: bool,
    display_plot_count: u8,
    // Plot names and whether or not they should be shown (painted)
    plot_names_show: Vec<(String, bool)>,
    ps_ui: PlotSettingsUi,
    log_start_date_settings: Vec<LogStartDateSettings>,
    mipmap_lvl: usize,
}

impl PlotSettings {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.visibility.toggle_visibility_ui(ui);
        if !self.log_start_date_settings.is_empty() {
            self.ps_ui.ui_toggle_show_filter(ui);
            if self.ps_ui.show_filter_settings {
                egui::Window::new(self.ps_ui.filter_settings_text())
                    .open(&mut self.ps_ui.show_filter_settings)
                    .show(ui.ctx(), |ui| {
                        let mut enable_all = false;
                        let mut disable_all = false;
                        egui::Grid::new("global_filter_settings").show(ui, |ui| {
                            if ui
                                .button(RichText::new("Show all").strong().heading())
                                .clicked()
                            {
                                enable_all = true;
                            }
                            if ui
                                .button(RichText::new("Hide all").strong().heading())
                                .clicked()
                            {
                                disable_all = true;
                            }
                        });
                        if enable_all {
                            for (_, show) in &mut *self.plot_names_show {
                                *show = true;
                            }
                        } else if disable_all {
                            for (_, show) in &mut *self.plot_names_show {
                                *show = false;
                            }
                        }

                        for (pname, show) in &mut self.plot_names_show {
                            ui.toggle_value(show, pname.as_str());
                        }
                    });
                if ui.ctx().input(|i| i.key_pressed(Key::Escape)) {
                    self.ps_ui.show_filter_settings = false;
                }
            }

            let show_loaded_logs_text = RichText::new(format!(
                "{} Loaded logs",
                if self.ps_ui.show_loaded_logs {
                    regular::EYE
                } else {
                    regular::EYE_SLASH
                }
            ));
            ui.toggle_value(
                &mut self.ps_ui.show_loaded_logs,
                show_loaded_logs_text.text(),
            );
            if self.ps_ui.show_loaded_logs {
                // Only react on Escape input if no settings are currently open
                if ui.ctx().input(|i| i.key_pressed(Key::Escape))
                    && !self.log_start_date_settings.iter().any(|s| s.clicked)
                {
                    self.ps_ui.show_loaded_logs = false;
                }
                egui::Window::new(show_loaded_logs_text)
                    .open(&mut self.ps_ui.show_loaded_logs)
                    .show(ui.ctx(), |ui| {
                        egui::Grid::new("log_settings_grid").show(ui, |ui| {
                            for settings in &mut self.log_start_date_settings {
                                loaded_logs::log_date_settings_ui(ui, settings);
                                ui.end_row();
                            }
                        });
                    });
            }
            if ui
                .add(
                    egui::DragValue::new(&mut self.mipmap_lvl)
                        .speed(1)
                        .range(0..=32)
                        .suffix(" MipMap lvl"),
                )
                .changed()
            {
                log::info!("Mip map level changed to: {}", self.mipmap_lvl);
            }
        }
    }

    /// Needs to be called once (and only once!) per frame before querying for plot ui settings, such as
    /// how many plots to paint and more.
    pub fn refresh(&mut self, plots: &mut Plots) {
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
    pub fn add_plot_name_if_not_exists(&mut self, plot_name: &str) {
        if !self
            .plot_names_show
            .iter()
            .any(|(name, _)| name == plot_name)
        {
            self.plot_names_show.push((plot_name.to_owned(), true));
        }
    }

    pub fn plot_name_filter(&self) -> Vec<&str> {
        self.plot_names_show
            .iter()
            .filter_map(|(name, show)| if *show { None } else { Some((*name).as_str()) })
            .collect()
    }

    /// Get the next ID for a log, used for when a new log is loaded and added to the collection of logs and log settings
    pub fn next_log_id(&self) -> usize {
        (self.total_logs() + 1).into()
    }

    pub fn total_logs(&self) -> u16 {
        self.log_start_date_settings.len() as u16
    }

    pub fn add_log_setting(&mut self, log_settings: LogStartDateSettings) {
        self.log_start_date_settings.push(log_settings);
    }

    // The id filter specifies which plots belonging to which logs should not be painted on the plot ui.
    pub fn log_id_filter(&self) -> Vec<usize> {
        let mut log_id_filter: Vec<usize> = vec![];
        for settings in &self.log_start_date_settings {
            if !settings.show_log() {
                log_id_filter.push(settings.log_id());
            }
        }
        log_id_filter
    }

    fn update_plot_dates(&mut self, plots: &mut Plots) {
        for settings in &mut self.log_start_date_settings {
            date_settings::update_plot_dates(&mut self.invalidate_plot, plots, settings);
        }
    }

    /// Returns true if changes in plot settings occurred such that various cached values
    /// related to plot layout needs to be recalculated.
    pub fn cached_plots_invalidated(&self) -> bool {
        self.invalidate_plot
    }

    pub fn mipmap_lvl(&self) -> usize {
        self.mipmap_lvl
    }
}
