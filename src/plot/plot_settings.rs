use plot_visibility_config::PlotVisibilityConfig;
use serde::{Deserialize, Serialize};

mod plot_visibility_config;

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct PlotSettings {
    visibility: PlotVisibilityConfig,
}

impl PlotSettings {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.visibility.toggle_visibility_ui(ui);
    }

    pub fn display_percentage(&self, percentage_plots_empty: bool) -> bool {
        self.visibility
            .should_display_percentage(percentage_plots_empty)
    }

    pub fn display_hundreds(&self, hundreds_plots_empty: bool) -> bool {
        self.visibility
            .should_display_hundreds(hundreds_plots_empty)
    }

    pub fn display_thousands(&self, thousands_plots_empty: bool) -> bool {
        self.visibility
            .should_display_thousands(thousands_plots_empty)
    }
}
