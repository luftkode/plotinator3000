use egui_phosphor::regular;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PlotVisibilityConfig {
    show_percentage_plot: bool,
    hovered_show_percentage_plot: bool,
    show_to_hundreds_plot: bool,
    hovered_show_to_hundreds_plot: bool,
    show_to_thousands_plot: bool,
    hovered_show_to_thousands_plot: bool,
}

impl Default for PlotVisibilityConfig {
    fn default() -> Self {
        Self {
            show_percentage_plot: true,
            hovered_show_percentage_plot: false,
            show_to_hundreds_plot: true,
            hovered_show_to_hundreds_plot: false,
            show_to_thousands_plot: true,
            hovered_show_to_thousands_plot: false,
        }
    }
}

impl PlotVisibilityConfig {
    pub fn should_display_percentage(&self, percentage_plots_empty: bool) -> bool {
        !percentage_plots_empty && self.show_percentage_plot
    }

    pub fn hovered_display_percentage(&self) -> bool {
        self.hovered_show_percentage_plot
    }

    pub fn should_display_hundreds(&self, hundreds_plots_empty: bool) -> bool {
        !hundreds_plots_empty && self.show_to_hundreds_plot
    }

    pub fn hovered_display_to_hundreds(&self) -> bool {
        self.hovered_show_to_hundreds_plot
    }

    pub fn should_display_thousands(&self, thousands_plots_empty: bool) -> bool {
        !thousands_plots_empty && self.show_to_thousands_plot
    }

    pub fn hovered_display_thousands(&self) -> bool {
        self.hovered_show_to_thousands_plot
    }

    pub fn toggle_visibility_ui(&mut self, ui: &mut egui::Ui) {
        self.hovered_show_percentage_plot = false;
        self.hovered_show_to_hundreds_plot = false;
        self.hovered_show_to_thousands_plot = false;

        let show_perc_plot_text = format!(
            "{icon} % plot",
            icon = if self.show_percentage_plot {
                regular::EYE
            } else {
                regular::EYE_SLASH
            }
        );
        if ui
            .toggle_value(&mut self.show_percentage_plot, show_perc_plot_text)
            .hovered()
        {
            self.hovered_show_percentage_plot = true;
        };

        let show_to_hundr_plot_text = format!(
            "{icon} 0-100 plot",
            icon = if self.show_to_hundreds_plot {
                regular::EYE
            } else {
                regular::EYE_SLASH
            }
        );
        if ui
            .toggle_value(&mut self.show_to_hundreds_plot, show_to_hundr_plot_text)
            .hovered()
        {
            self.hovered_show_to_hundreds_plot = true;
        };

        let show_to_thousands_plot_text = format!(
            "{icon} 0-1000s plot",
            icon = if self.show_to_thousands_plot {
                regular::EYE
            } else {
                regular::EYE_SLASH
            }
        );
        if ui
            .toggle_value(
                &mut self.show_to_thousands_plot,
                show_to_thousands_plot_text,
            )
            .hovered()
        {
            self.hovered_show_to_thousands_plot = true;
        };
    }
}
