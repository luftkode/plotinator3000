use egui_phosphor::regular::{ARROWS_OUT_LINE_VERTICAL, CHART_LINE, CHART_SCATTER, LINE_SEGMENTS};
use plotinator_plot_util::draw_series::SeriesDrawMode;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Deserialize, Serialize, Clone, Copy)]
pub struct SeriesPlotSettings {
    // The hovered mode takes precedence, to let the user easily preview the other settings
    hovered_draw_mode: Option<SeriesDrawMode>,
    draw_mode: SeriesDrawMode,
    width: f32,
}

impl Default for SeriesPlotSettings {
    fn default() -> Self {
        Self {
            hovered_draw_mode: Default::default(),
            draw_mode: Default::default(),
            width: 1.5,
        }
    }
}

impl SeriesPlotSettings {
    /// Render the [`SeriesPlotSettings`] part of the UI
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let line_with_emphasis_label = format!("{LINE_SEGMENTS} Line");
        let line_with_emphasis_label_long = format!("{LINE_SEGMENTS} Auto-emphasis Line");
        let line_label = format!("{CHART_LINE} Line");
        let scatter_label = format!("{CHART_SCATTER} Scatter");

        let menu_button_label = match self.draw_mode {
            SeriesDrawMode::LineWithEmphasis { .. } => line_with_emphasis_label,
            SeriesDrawMode::Line { .. } => line_label.clone(),
            SeriesDrawMode::Scatter { .. } => scatter_label.clone(),
        };

        self.hovered_draw_mode = None;
        ui.menu_button(menu_button_label, |ui| {
            let button = ui
                .button(line_with_emphasis_label_long)
                .on_hover_text("Line with points highlighted when spacing exceeds threshold");
            if button.clicked() {
                self.draw_mode = SeriesDrawMode::LineWithEmphasis { width: 0.0 };
            } else if button.hovered() {
                self.hovered_draw_mode = Some(SeriesDrawMode::LineWithEmphasis { width: 0.0 });
            }
            let button = ui.button(line_label).on_hover_text("Connected line plot");
            if button.clicked() {
                self.draw_mode = SeriesDrawMode::Line { width: 0.0 };
            } else if button.hovered() {
                self.hovered_draw_mode = Some(SeriesDrawMode::Line { width: 0.0 });
            }

            let button = ui
                .button(scatter_label)
                .on_hover_text("Individual points only");
            if button.clicked() {
                self.draw_mode = SeriesDrawMode::Scatter { width: 0.0 };
            } else if button.hovered() {
                self.hovered_draw_mode = Some(SeriesDrawMode::Scatter { width: 0.0 });
            }
        })
        .response
        .on_hover_text("Choose how to display data series");
        ui.label(format!("{ARROWS_OUT_LINE_VERTICAL} width"));
        ui.add(
            egui::DragValue::new(&mut self.width)
                .speed(0.02)
                .range(0.5..=20.0),
        );
    }

    /// Return the selected [`SeriesDrawMode`].
    pub fn draw_mode(&self) -> SeriesDrawMode {
        let width = self.width;

        match self.hovered_draw_mode.unwrap_or(self.draw_mode) {
            SeriesDrawMode::LineWithEmphasis { .. } => SeriesDrawMode::LineWithEmphasis { width },
            SeriesDrawMode::Line { .. } => SeriesDrawMode::Line { width },
            SeriesDrawMode::Scatter { .. } => SeriesDrawMode::Scatter { width },
        }
    }
}
