use chrono::{DateTime, Utc};
use egui::{Order, RichText};
use plotinator_ui_util::{date_editor::DateEditor, number_editor::NumberEditor};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Deserialize, Serialize)]
pub(crate) struct CutOutsideMinMaxRange {
    pub(crate) x_range: (f64, f64),
    pub(crate) y_min_max: (f64, f64),
}

impl CutOutsideMinMaxRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>, (min, max): (f64, f64)) -> Self {
        Self {
            x_range: (
                start.timestamp_nanos_opt().expect("invalid time") as f64,
                end.timestamp_nanos_opt().expect("invalid time") as f64,
            ),
            y_min_max: (min, max),
        }
    }
}

#[derive(PartialEq, Default, Deserialize, Serialize)]
pub(crate) struct LogPointsCutter {
    pub(crate) clicked: bool,
    pub(crate) start_date: DateEditor,
    pub(crate) end_date: DateEditor,
    pub(crate) min_val: NumberEditor,
    pub(crate) max_val: NumberEditor,
    pub(crate) cut_points_x_range: Option<(f64, f64)>,
    pub(crate) cut_points_outside_minmax: Option<CutOutsideMinMaxRange>,
}

impl LogPointsCutter {
    pub fn set_cut_points_x_range(&mut self, start: DateTime<Utc>, end: DateTime<Utc>) {
        self.cut_points_x_range = Some((
            start.timestamp_nanos_opt().expect("invalid time") as f64,
            end.timestamp_nanos_opt().expect("invalid time") as f64,
        ));
    }
}

impl LogPointsCutter {
    pub fn show(&mut self, ui: &egui::Ui, log_name_date: &str) {
        let mut open = true;
        egui::Window::new(
            RichText::new(format!("Cutting {log_name_date}"))
                .size(20.0)
                .strong(),
        )
        .collapsible(false)
        .movable(false)
        .open(&mut open)
        .order(Order::Foreground)
        .default_size([450.0, 400.0])
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
        .show(ui.ctx(), |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.label("Remove range");
                self.start_date.show(ui);
                self.end_date.show(ui);

                ui.horizontal(|ui| {
                    self.min_val.show(ui);
                    self.max_val.show(ui);
                });

                if ui.button("Remove points in range").clicked() {
                    if let (Some(start), Some(end)) =
                        (self.start_date.current(), self.end_date.current())
                    {
                        log::info!(
                            "Removing points in range: {} - {}",
                            start.format("%Y-%m-%d %H%M%S"),
                            end.format("%Y-%m-%d %H%M%S")
                        );

                        self.set_cut_points_x_range(start, end);
                    }
                }
                if ui
                    .button("Remove points in range but outside min/max")
                    .clicked()
                {
                    if let (Some(min), Some(max)) = (self.min_val.current(), self.max_val.current())
                    {
                        if let (Some(start), Some(end)) =
                            (self.start_date.current(), self.end_date.current())
                        {
                            log::info!(
                                "Removing points in range: {} - {}, but outside {min} - {max}",
                                start.format("%Y-%m-%d %H%M%S"),
                                end.format("%Y-%m-%d %H%M%S")
                            );
                            self.cut_points_outside_minmax =
                                Some(CutOutsideMinMaxRange::new(start, end, (min, max)));
                        }
                    }
                }
            })
        });

        if !open {
            self.clicked = false;
        }
    }
}
