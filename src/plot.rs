use crate::logs::{
    pid::{PidLog, PidLogEntry},
    status::{StatusLog, StatusLogEntry},
    LogEntry,
};
use egui::Response;
use egui_plot::{Corner, HPlacement, Legend, Line, Plot, PlotPoints};

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
struct AxisLink {
    link_x: bool,
    link_cursor_x: bool,
}

impl Default for AxisLink {
    fn default() -> Self {
        Self {
            link_x: true,
            link_cursor_x: true,
        }
    }
}

#[derive(Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LogPlot {
    config: Legend,
    line_width: f32,
    axis_link: AxisLink,
}

impl LogPlot {
    fn line_from_log_entry<F, L: LogEntry>(pid_logs: &[L], y_extractor: F) -> Line
    where
        F: Fn(&L) -> f64,
    {
        let points: PlotPoints = pid_logs
            .iter()
            .map(|e| {
                let x = e.timestamp_ms() as f64;
                let y = y_extractor(e);
                [x, y]
            })
            .collect();
        Line::new(points)
    }

    fn pid_log_lines(pid_logs: &[PidLogEntry]) -> (Vec<Line>, Vec<Line>) {
        let zero_to_one_range = vec![
            Self::line_from_log_entry(pid_logs, |e| e.pid_err as f64).name("PID Error"),
            Self::line_from_log_entry(pid_logs, |e| e.servo_duty_cycle as f64)
                .name("Servo Duty Cycle"),
        ];
        let big_range = vec![Self::line_from_log_entry(pid_logs, |e| e.rpm as f64).name("RPM")];
        (zero_to_one_range, big_range)
    }

    fn status_log_lines(status_log: &[StatusLogEntry]) -> (Vec<Line>, Vec<Line>) {
        let zero_to_one_range =
            vec![Self::line_from_log_entry(status_log, |e| (e.fan_on as u8) as f64).name("Fan On")];

        let big_range = vec![
            Self::line_from_log_entry(status_log, |e| e.engine_temp as f64).name("Engine Temp °C"),
            Self::line_from_log_entry(status_log, |e| e.vbat.into()).name("Vbat"),
            Self::line_from_log_entry(status_log, |e| e.motor_state.into()).name("Motor State"),
            Self::line_from_log_entry(status_log, |e| e.setpoint.into()).name("Setpoint"),
        ];
        (zero_to_one_range, big_range)
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        pid_log: Option<&PidLog>,
        status_log: Option<&StatusLog>,
    ) -> Response {
        let Self {
            config,
            line_width,
            axis_link,
        } = self;

        egui::Grid::new("settings").show(ui, |ui| {
            ui.label("Text style:");
            ui.horizontal(|ui| {
                let all_text_styles = ui.style().text_styles();
                for style in all_text_styles {
                    ui.selectable_value(&mut config.text_style, style.clone(), style.to_string());
                }
            });

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
            ui.label("Line width");
            ui.add(
                egui::DragValue::new(line_width)
                    .speed(0.02)
                    .range(0.5..=20.0),
            );
            ui.horizontal(|ui| {
                ui.label("Linked axes:");
                ui.checkbox(&mut self.axis_link.link_x, "X");
                ui.label("Linked cursors:");
                ui.checkbox(&mut self.axis_link.link_cursor_x, "X");
            });
            ui.end_row();
        });
        let link_group_id = ui.id().with("linked_plots");
        ui.vertical(|ui| {
            let plot_height = ui.available_height() / 2.0;

            let zero_to_one_range_plot = Plot::new("zero_to_one_range_plot")
                .legend(config.clone())
                .height(plot_height)
                .y_axis_position(HPlacement::Right)
                .link_axis(link_group_id, self.axis_link.link_x, false)
                .link_cursor(link_group_id, self.axis_link.link_cursor_x, false);

            // Plot for values outside 0-1 range
            let large_range_plot = Plot::new("large_range_plot")
                .legend(config.clone())
                .height(plot_height)
                .y_axis_position(HPlacement::Right)
                .link_axis(link_group_id, self.axis_link.link_x, false)
                .link_cursor(link_group_id, self.axis_link.link_cursor_x, false);

            zero_to_one_range_plot.show(ui, |plot_ui| {
                if let Some(log) = pid_log {
                    let (zero_to_one_range, _) = Self::pid_log_lines(log.entries());
                    for lineplot in zero_to_one_range {
                        plot_ui.line(lineplot.width(*line_width));
                    }
                }
                if let Some(log) = status_log {
                    let (zero_to_one_range, _) = Self::status_log_lines(log.entries());
                    for lineplot in zero_to_one_range {
                        plot_ui.line(lineplot.width(*line_width));
                    }
                }
            });

            large_range_plot.show(ui, |plot_ui| {
                if let Some(log) = pid_log {
                    let (_, large_range) = Self::pid_log_lines(log.entries());
                    for lineplot in large_range {
                        plot_ui.line(lineplot.width(*line_width));
                    }
                }
                if let Some(log) = status_log {
                    let (_, large_range) = Self::status_log_lines(log.entries());
                    for lineplot in large_range {
                        plot_ui.line(lineplot.width(*line_width));
                    }
                }
            });
        })
        .response
    }
}
