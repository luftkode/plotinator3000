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
    visibility: PlotVisibilityConfig,
    display_percentage_plot: bool,
    display_hundreds_plot: bool,
    display_thousands_plot: bool,
    display_plot_count: u8,
    // Plot names and whether or not they should be shown (painted)
    plot_names_show: Vec<(String, bool)>,
    ps_ui: PlotSettingsUi,
    log_start_date_settings: Vec<LogStartDateSettings>,
}

impl PlotSettings {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        log_start_date_settings: &mut [LogStartDateSettings],
    ) {
        self.visibility.toggle_visibility_ui(ui);
        if !log_start_date_settings.is_empty() {
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
                    && !log_start_date_settings.iter().any(|s| s.clicked)
                {
                    self.ps_ui.show_loaded_logs = false;
                }
                egui::Window::new(show_loaded_logs_text)
                    .open(&mut self.ps_ui.show_loaded_logs)
                    .show(ui.ctx(), |ui| {
                        egui::Grid::new("log_settings_grid").show(ui, |ui| {
                            for settings in log_start_date_settings {
                                loaded_logs::log_date_settings_ui(ui, settings);
                                ui.end_row();
                            }
                        });
                    });
            }
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
        total_plot_count += self.display_hundreds_plot as u8;
        self.display_plot_count = total_plot_count;
    }

    pub fn plot_name_show_mut(&mut self) -> &mut Vec<(String, bool)> {
        &mut self.plot_names_show
    }

    pub fn plot_name_filter(&self) -> Vec<&str> {
        self.plot_names_show
            .iter()
            .filter_map(|(name, show)| if *show { None } else { Some((*name).as_str()) })
            .collect()
    }
}
