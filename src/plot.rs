use std::ops::RangeInclusive;

use crate::{
    app::PlayBackButtonEvent,
    logs::{
        generator::GeneratorLog,
        mbed_motor_control::{pid::PidLog, status::StatusLog},
    },
    util::format_ms_timestamp,
};
use axis_config::{AxisConfig, PlotType};
use chrono::{DateTime, Timelike};
use egui::{Color32, Response, RichText};
use egui_plot::{AxisHints, GridMark, HPlacement, Legend, Line, Plot, PlotPoint, Text, VPlacement};
use play_state::{playback_update_generator_plot, playback_update_plot, PlayState};
use plot_visibility_config::PlotVisibilityConfig;
use util::{ExpectedPlotRange, PlotWithName};

mod axis_config;
pub mod mipmap;
mod play_state;
mod plot_visibility_config;
pub mod util;

#[derive(PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LogPlot {
    config: Legend,
    line_width: f32,
    axis_config: AxisConfig,
    play_state: PlayState,
    percentage_plots: Vec<PlotWithName>,
    to_hundreds_plots: Vec<PlotWithName>,
    to_thousands_plots: Vec<PlotWithName>,
    plot_visibility: PlotVisibilityConfig,
}

impl Default for LogPlot {
    fn default() -> Self {
        Self {
            config: Default::default(),
            line_width: 1.5,
            axis_config: Default::default(),
            play_state: PlayState::default(),
            percentage_plots: vec![],
            to_hundreds_plots: vec![],
            to_thousands_plots: vec![],
            plot_visibility: PlotVisibilityConfig::default(),
        }
    }
}

impl LogPlot {
    pub fn formatted_playback_time(&self) -> String {
        self.play_state.formatted_time()
    }
    pub fn is_playing(&self) -> bool {
        self.play_state.is_playing()
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        pid_log: Option<&PidLog>,
        status_log: Option<&StatusLog>,
        generator_log: Option<&GeneratorLog>,
    ) -> Response {
        let Self {
            config,
            line_width,
            axis_config,
            play_state,
            percentage_plots,
            to_hundreds_plots,
            to_thousands_plots,
            plot_visibility,
        } = self;

        let mut playback_button_event = None;

        Self::show_settings_grid(
            ui,
            play_state,
            &mut playback_button_event,
            line_width,
            axis_config,
            plot_visibility,
        );
        if let Some(e) = playback_button_event {
            play_state.handle_playback_button_press(e);
        };
        let is_reset_pressed = matches!(playback_button_event, Some(PlayBackButtonEvent::Reset));
        let timer = play_state.time_since_update();
        let link_group_id = ui.id().with("linked_plots");

        ui.vertical(|ui| {
            if let Some(pid_log) = pid_log {
                for (points, name, range) in pid_log.all_plots_raw().iter() {
                    match range {
                        ExpectedPlotRange::Percentage => {
                            if !percentage_plots.iter().any(|p| p.name == *name) {
                                percentage_plots
                                    .push(PlotWithName::new(points.clone(), name.clone()))
                            }
                        }
                        ExpectedPlotRange::OneToOneHundred => {
                            if !to_hundreds_plots.iter().any(|p| p.name == *name) {
                                to_hundreds_plots
                                    .push(PlotWithName::new(points.clone(), name.clone()))
                            }
                        }
                        ExpectedPlotRange::Thousands => {
                            if !to_thousands_plots.iter().any(|p| p.name == *name) {
                                to_thousands_plots
                                    .push(PlotWithName::new(points.clone(), name.clone()))
                            }
                        }
                    }
                }
            }
            if let Some(status_log) = status_log {
                for (points, name, range) in status_log.all_plots_raw().iter() {
                    match range {
                        ExpectedPlotRange::Percentage => {
                            if !percentage_plots.iter().any(|p| p.name == *name) {
                                percentage_plots
                                    .push(PlotWithName::new(points.clone(), name.clone()))
                            }
                        }
                        ExpectedPlotRange::OneToOneHundred => {
                            if !to_hundreds_plots.iter().any(|p| p.name == *name) {
                                to_hundreds_plots
                                    .push(PlotWithName::new(points.clone(), name.clone()))
                            }
                        }
                        ExpectedPlotRange::Thousands => {
                            if !to_thousands_plots.iter().any(|p| p.name == *name) {
                                to_thousands_plots
                                    .push(PlotWithName::new(points.clone(), name.clone()))
                            }
                        }
                    }
                }
            }
            // Calculate the number of plots to display
            let mut total_plot_count: u8 = 0;
            let display_percentage_plot =
                plot_visibility.should_display_percentage(percentage_plots);
            total_plot_count += display_percentage_plot as u8;
            let display_to_hundred_plot =
                plot_visibility.should_display_to_hundreds(to_hundreds_plots);
            total_plot_count += display_to_hundred_plot as u8;
            let display_to_thousands_plot =
                plot_visibility.should_display_to_thousands(to_thousands_plots);
            total_plot_count += display_to_thousands_plot as u8;

            if generator_log.is_some() {
                total_plot_count += 1;
            }

            let plot_height = ui.available_height() / (total_plot_count as f32);

            let create_plot = |name: &str| {
                Plot::new(name)
                    .legend(config.clone())
                    .height(plot_height)
                    .show_axes(axis_config.show_axes())
                    .y_axis_position(HPlacement::Right)
                    .include_y(0.0)
                    .x_axis_formatter(move |x, _range| format_ms_timestamp(x.value))
                    .link_axis(link_group_id, axis_config.link_x(), false)
                    .link_cursor(link_group_id, axis_config.link_cursor_x(), false)
            };

            let percentage_plot = create_plot("percentage")
                .include_y(1.0)
                .y_axis_formatter(|y, _range| format!("{:.0}%", y.value * 100.0));

            let to_hundred = create_plot("to_hundreds");
            let thousands = create_plot("to_thousands");

            if display_percentage_plot {
                percentage_plot.show(ui, |plot_ui| {
                    Self::handle_plot(plot_ui, |plot_ui| {
                        if let Some(status_log) = status_log {
                            for (ts, st_change) in status_log.timestamps_with_state_changes() {
                                plot_ui.text(Text::new(
                                    PlotPoint::new(*ts as f64, ((*st_change as u8) as f64) / 10.0),
                                    st_change.to_string(),
                                ));
                            }
                        }
                        for plot_with_name in percentage_plots {
                            let line = Line::new(plot_with_name.raw_plot.to_vec())
                                .name(plot_with_name.name.to_owned());
                            plot_ui.line(line.width(*line_width));
                        }
                        playback_update_plot(timer, plot_ui, is_reset_pressed);
                        axis_config.handle_y_axis_lock(plot_ui, PlotType::Percentage, |plot_ui| {
                            playback_update_plot(timer, plot_ui, is_reset_pressed)
                        });
                    });
                });
            }

            if display_to_hundred_plot {
                ui.separator();
                to_hundred.show(ui, |plot_ui| {
                    Self::handle_plot(plot_ui, |plot_ui| {
                        for plot_with_name in to_hundreds_plots {
                            let line = Line::new(plot_with_name.raw_plot.to_vec())
                                .name(plot_with_name.name.to_owned());
                            plot_ui.line(line.width(*line_width));
                        }
                        axis_config.handle_y_axis_lock(plot_ui, PlotType::Hundreds, |plot_ui| {
                            playback_update_plot(timer, plot_ui, is_reset_pressed)
                        });
                    });
                });
            }

            if display_to_thousands_plot {
                ui.separator();
                thousands.show(ui, |plot_ui| {
                    Self::handle_plot(plot_ui, |plot_ui| {
                        for plot_with_name in to_thousands_plots {
                            let line = Line::new(plot_with_name.raw_plot.to_vec())
                                .name(plot_with_name.name.to_owned());
                            plot_ui.line(line.width(*line_width));
                        }

                        if let Some(log) = status_log {
                            for (ts, st_change) in log.timestamps_with_state_changes() {
                                plot_ui.text(Text::new(
                                    PlotPoint::new(*ts as f64, (*st_change as u8) as f64),
                                    st_change.to_string(),
                                ))
                            }
                        }
                        axis_config.handle_y_axis_lock(plot_ui, PlotType::Thousands, |plot_ui| {
                            playback_update_plot(timer, plot_ui, is_reset_pressed)
                        });
                    });
                });
            }

            if let Some(gen_log) = generator_log {
                ui.separator();
                let time_formatter = |mark: GridMark, _range: &RangeInclusive<f64>| {
                    let sec = mark.value;
                    let dt = DateTime::from_timestamp(sec as i64, 0).unwrap();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                };
                let x_axes = vec![AxisHints::new_x().label("Time").formatter(time_formatter)];
                let label_fmt = |_s: &str, val: &PlotPoint| {
                    let dt = DateTime::from_timestamp(val.x as i64, 0).unwrap();
                    format!(
                        "{h:02}:{m:02}:{s:02}",
                        h = dt.hour(),
                        m = dt.minute(),
                        s = dt.second()
                    )
                };

                let gen_log_plot = Plot::new("generator_log_plot")
                    .legend(config.clone())
                    .height(plot_height)
                    .show_axes(axis_config.show_axes())
                    .x_axis_position(VPlacement::Top)
                    .y_axis_position(HPlacement::Right)
                    .custom_x_axes(x_axes)
                    .label_formatter(label_fmt)
                    .include_y(0.0);

                gen_log_plot.show(ui, |plot_ui| {
                    Self::handle_plot(plot_ui, |plot_ui| {
                        for line_plot in gen_log.all_plots() {
                            plot_ui.line(line_plot.width(*line_width));
                        }
                        axis_config.handle_y_axis_lock(plot_ui, PlotType::Generator, |plot_ui| {
                            playback_update_generator_plot(
                                timer,
                                plot_ui,
                                is_reset_pressed,
                                gen_log.first_timestamp().unwrap_or(0.0),
                            )
                        });
                    });
                });
            }
        })
        .response
    }

    fn handle_plot<F>(plot_ui: &mut egui_plot::PlotUi, plot_function: F)
    where
        F: FnOnce(&mut egui_plot::PlotUi),
    {
        plot_function(plot_ui);
    }

    fn show_settings_grid(
        ui: &mut egui::Ui,
        play_state: &PlayState,
        playback_button_event: &mut Option<PlayBackButtonEvent>,
        line_width: &mut f32,
        axis_cfg: &mut AxisConfig,
        plot_visibility_cfg: &mut PlotVisibilityConfig,
    ) {
        egui::Grid::new("settings").show(ui, |ui| {
            ui.label("Line width");
            ui.add(
                egui::DragValue::new(line_width)
                    .speed(0.02)
                    .range(0.5..=20.0),
            );
            ui.horizontal_top(|ui| {
                axis_cfg.toggle_axis_cfg_ui(ui);
                ui.label("|");
                plot_visibility_cfg.toggle_visibility_ui(ui);
            });

            ui.horizontal_centered(|ui| {
                ui.label("| ");
                // Reset button
                let reset_text = RichText::new(egui_phosphor::regular::REWIND);
                if ui.button(reset_text).clicked() {
                    *playback_button_event = Some(PlayBackButtonEvent::Reset);
                }
                let playpause_text = if play_state.is_playing() {
                    RichText::new(egui_phosphor::regular::PAUSE).color(Color32::YELLOW)
                } else {
                    RichText::new(egui_phosphor::regular::PLAY).color(Color32::GREEN)
                };
                if ui.button(playpause_text).clicked() {
                    *playback_button_event = Some(PlayBackButtonEvent::PlayPause);
                }

                ui.label(RichText::new(play_state.formatted_time()));
                ui.label(" |");
            });

            ui.end_row();
        });
    }
}
