use crate::logs::{
    pid::{PidLog, PidLogEntry},
    status::{StatusLog, StatusLogEntry},
};
use egui::Response;
use egui_plot::{Corner, Legend, Line, Plot, PlotPoints};

#[derive(Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LogPlot {
    config: Legend,
    line_width: f32,
}

impl LogPlot {
    fn pid_log_rpm(pid_logs: &[PidLogEntry]) -> Line {
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

    fn pid_log_pid_err(pid_logs: &[PidLogEntry]) -> Line {
        let points: PlotPoints = pid_logs
            .iter()
            .map(|e| {
                let x = e.timestamp_ms as f64;
                let y = e.pid_err as f64;
                [x, y]
            })
            .collect();

        Line::new(points)
    }

    fn pid_log_servo_duty_cycle(pid_logs: &[PidLogEntry]) -> Line {
        let points: PlotPoints = pid_logs
            .iter()
            .map(|e| {
                let x = e.timestamp_ms as f64;
                let y = e.servo_duty_cycle as f64;
                [x, y]
            })
            .collect();

        Line::new(points)
    }

    fn status_log_engine_temp(status_log: &[StatusLogEntry]) -> Line {
        let points: PlotPoints = status_log
            .iter()
            .map(|e| {
                let x = e.timestamp_ms as f64;
                let y = e.engine_temp as f64;
                [x, y]
            })
            .collect();

        Line::new(points)
    }

    fn status_log_fan_on(status_log: &[StatusLogEntry]) -> Line {
        let points: PlotPoints = status_log
            .iter()
            .map(|e| {
                let x = e.timestamp_ms as f64;
                let y = (e.fan_on as u8) as f64;
                [x, y]
            })
            .collect();

        Line::new(points)
    }

    fn status_log_vbat(status_log: &[StatusLogEntry]) -> Line {
        let points: PlotPoints = status_log
            .iter()
            .map(|e| {
                let x = e.timestamp_ms as f64;
                let y = e.vbat as f64;
                [x, y]
            })
            .collect();

        Line::new(points)
    }

    fn status_log_setpoint(status_log: &[StatusLogEntry]) -> Line {
        let points: PlotPoints = status_log
            .iter()
            .map(|e| {
                let x = e.timestamp_ms as f64;
                let y = e.setpoint as f64;
                [x, y]
            })
            .collect();

        Line::new(points)
    }

    fn status_log_motorstate(status_log: &[StatusLogEntry]) -> Line {
        let points: PlotPoints = status_log
            .iter()
            .map(|e| {
                let x = e.timestamp_ms as f64;
                let y = e.motor_state as f64;
                [x, y]
            })
            .collect();

        Line::new(points)
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        pid_log: Option<&PidLog>,
        status_log: Option<&StatusLog>,
    ) -> Response {
        let Self { config, line_width } = self;

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
            ui.label("Line width");
            ui.add(
                egui::DragValue::new(line_width)
                    .speed(0.02)
                    .range(0.5..=20.0),
            );
            ui.end_row();
        });
        let legend_plot = Plot::new("plots").legend(config.clone());
        legend_plot
            .show(ui, |plot_ui| {
                if let Some(log) = pid_log {
                    plot_ui.line(
                        Self::pid_log_rpm(log.entries())
                            .name("RPM")
                            .width(*line_width),
                    );
                    plot_ui.line(
                        Self::pid_log_pid_err(log.entries())
                            .name("PID Error")
                            .width(*line_width),
                    );
                    plot_ui.line(
                        Self::pid_log_servo_duty_cycle(log.entries())
                            .name("Servo Duty Cycle")
                            .width(*line_width),
                    );
                }
                if let Some(log) = status_log {
                    plot_ui.line(
                        Self::status_log_engine_temp(log.entries())
                            .name("Engine temp")
                            .width(*line_width),
                    );
                    plot_ui.line(
                        Self::status_log_fan_on(log.entries())
                            .name("Fan On")
                            .width(*line_width),
                    );
                    plot_ui.line(
                        Self::status_log_vbat(log.entries())
                            .name("Vbat")
                            .width(*line_width),
                    );
                    plot_ui.line(
                        Self::status_log_setpoint(log.entries())
                            .name("Setpoint")
                            .width(*line_width),
                    );
                    plot_ui.line(
                        Self::status_log_motorstate(log.entries())
                            .name("Motor State")
                            .width(*line_width),
                    );
                }
            })
            .response
    }
}
