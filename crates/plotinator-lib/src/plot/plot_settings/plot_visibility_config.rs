use egui_phosphor::regular;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PlotVisibilityConfig {
    show_percentage_plot: bool,
    show_to_hundreds_plot: bool,
    show_to_thousands_plot: bool,
}

impl Default for PlotVisibilityConfig {
    fn default() -> Self {
        Self {
            show_percentage_plot: true,
            show_to_hundreds_plot: true,
            show_to_thousands_plot: true,
        }
    }
}

impl PlotVisibilityConfig {
    pub fn should_display_percentage(&self, percentage_plots_empty: bool) -> bool {
        !percentage_plots_empty && self.show_percentage_plot
    }

    pub fn should_display_hundreds(&self, hundreds_plots_empty: bool) -> bool {
        !hundreds_plots_empty && self.show_to_hundreds_plot
    }

    pub fn should_display_thousands(&self, thousands_plots_empty: bool) -> bool {
        !thousands_plots_empty && self.show_to_thousands_plot
    }

    pub fn toggle_visibility_ui(&mut self, ui: &mut egui::Ui) {
        let show_perc_plot_text = format!(
            "{} % plot",
            if self.show_percentage_plot {
                regular::EYE
            } else {
                regular::EYE_SLASH
            }
        );
        ui.toggle_value(&mut self.show_percentage_plot, show_perc_plot_text);
        let show_to_hundr_plot_text = format!(
            "{} 0-100 plot",
            if self.show_to_hundreds_plot {
                regular::EYE
            } else {
                regular::EYE_SLASH
            }
        );

        ui.toggle_value(&mut self.show_to_hundreds_plot, show_to_hundr_plot_text);
        let show_to_thousands_plot_text = format!(
            "{} 0-1000s plot",
            if self.show_to_thousands_plot {
                regular::EYE
            } else {
                regular::EYE_SLASH
            }
        );
        ui.toggle_value(
            &mut self.show_to_thousands_plot,
            show_to_thousands_plot_text,
        );
    }
}
