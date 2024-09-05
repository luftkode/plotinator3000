use crate::logs::pid::PidLogEntry;
use egui::Response;
use egui_plot::{Corner, Legend, Line, Plot, PlotPoints};

#[derive(Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LogPlot {
    config: Legend,
}

impl LogPlot {
    fn pid_log(pid_logs: &[PidLogEntry]) -> Line {
        let points: PlotPoints = pid_logs
            .iter()
            .map(|e| {
                let x = e.timestamp_ms as f64;
                let y = e.rpm as f64;
                [x, y]
            })
            .collect();

        Line::new(points)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, pid_logs: Option<&[PidLogEntry]>) -> Response {
        let Self { config } = self;

        egui::Grid::new("settings").show(ui, |ui| {
            ui.label("Text style:");
            ui.horizontal(|ui| {
                let all_text_styles = ui.style().text_styles();
                for style in all_text_styles {
                    ui.selectable_value(&mut config.text_style, style.clone(), style.to_string());
                }
            });
            ui.end_row();

            ui.label("Position:");
            ui.horizontal(|ui| {
                Corner::all().for_each(|position| {
                    ui.selectable_value(&mut config.position, position, format!("{position:?}"));
                });
            });
            ui.end_row();

            ui.label("Opacity:");
            ui.add(
                egui::DragValue::new(&mut config.background_alpha)
                    .speed(0.02)
                    .range(0.0..=1.0),
            );
            ui.end_row();
        });
        let legend_plot = Plot::new("plots").legend(config.clone()).data_aspect(1.0);
        legend_plot
            .show(ui, |plot_ui| {
                if let Some(logs) = pid_logs {
                    plot_ui.line(Self::pid_log(logs).name("pid"));
                }
            })
            .response
    }
}
