use egui::{Response, RichText};
use egui_phosphor::regular::{ARROWS_OUT_LINE_VERTICAL, CHART_LINE, CHART_SCATTER, LINE_SEGMENTS};
use plotinator_plot_util::draw_series::SeriesDrawMode;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Deserialize, Serialize, Clone, Copy)]
pub struct SeriesPlotSettings {
    // The hovered mode takes precedence, to let the user easily preview the other settings
    hovered_draw_mode: Option<SeriesDrawMode>,
    draw_mode: SeriesDrawMode,
    line_width: f32,
}

impl Default for SeriesPlotSettings {
    fn default() -> Self {
        Self {
            hovered_draw_mode: Default::default(),
            draw_mode: Default::default(),
            line_width: 1.5,
        }
    }
}

impl SeriesPlotSettings {
    /// Render the [`SeriesPlotSettings`] part of the UI
    pub fn show(&mut self, ui: &mut egui::Ui) -> Response {
        let line_with_emphasis_label = RichText::new(format!("{LINE_SEGMENTS} Line"));
        let line_with_emphasis_label_long =
            RichText::new(format!("{LINE_SEGMENTS} Auto-emphasis Line"));
        let line_label = RichText::new(format!("{CHART_LINE} Line"));
        let scatter_label = RichText::new(format!("{CHART_SCATTER} Scatter"));

        let menu_button_label = match self.draw_mode {
            SeriesDrawMode::LineWithEmphasis => line_with_emphasis_label,
            SeriesDrawMode::Line => line_label.clone(),
            SeriesDrawMode::Scatter => scatter_label.clone(),
        };

        egui::Grid::new("Series Plot settings")
            .show(ui, |ui| {
                self.hovered_draw_mode = None;
                ui.menu_button(menu_button_label, |ui| {
                    let button = ui.button(line_with_emphasis_label_long).on_hover_text(
                        "Line with points highlighted when spacing exceeds threshold",
                    );
                    if button.clicked() {
                        self.draw_mode = SeriesDrawMode::LineWithEmphasis;
                    } else if button.hovered() {
                        self.hovered_draw_mode = Some(SeriesDrawMode::LineWithEmphasis);
                    }
                    let button = ui.button(line_label).on_hover_text("Connected line plot");
                    if button.clicked() {
                        self.draw_mode = SeriesDrawMode::Line;
                    } else if button.hovered() {
                        self.hovered_draw_mode = Some(SeriesDrawMode::Line);
                    }

                    let button = ui
                        .button(scatter_label)
                        .on_hover_text("Individual points only");
                    if button.clicked() {
                        self.draw_mode = SeriesDrawMode::Scatter;
                    } else if button.hovered() {
                        self.hovered_draw_mode = Some(SeriesDrawMode::Scatter);
                    }
                })
                .response
                .on_hover_text("Choose how to display data series");
                ui.label(format!("{ARROWS_OUT_LINE_VERTICAL} width"));
                ui.add(
                    egui::DragValue::new(&mut self.line_width)
                        .speed(0.02)
                        .range(0.5..=20.0),
                );
                ui.label("|");
            })
            .response
    }

    /// Return the selected [`SeriesDrawMode`].
    pub fn draw_mode(&self) -> SeriesDrawMode {
        self.hovered_draw_mode.unwrap_or(self.draw_mode)
    }

    pub fn line_width(&self) -> f32 {
        self.line_width
    }
}
