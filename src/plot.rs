use skytem_logs::{
    generator::GeneratorLog,
    mbed_motor_control::{pid::PidLog, status::StatusLog},
};
use std::ops::RangeInclusive;

use crate::{app::PlayBackButtonEvent, util::format_ms_timestamp};
use axis_config::{AxisConfig, PlotType};
use chrono::{DateTime, Timelike};
use egui::Response;
use egui_plot::{AxisHints, GridMark, HPlacement, Legend, Line, Plot, PlotPoint, Text, VPlacement};
use play_state::{playback_update_generator_plot, playback_update_plot, PlayState};
use plot_visibility_config::PlotVisibilityConfig;
use skytem_logs::plot_util::{ExpectedPlotRange, PlotWithName};

mod axis_config;
pub mod mipmap;
mod play_state;
mod plot_ui;
mod plot_visibility_config;

#[allow(missing_debug_implementations)] // Legend is from egui_plot and doesn't implement debug
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

    // TODO: Fix this lint
    #[allow(clippy::too_many_lines)]
    pub fn ui(
        &mut self,
        gui: &mut egui::Ui,
        pid_logs: &[PidLog],
        status_logs: &[StatusLog],
        generator_logs: &[GeneratorLog],
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

        plot_ui::show_settings_grid(
            gui,
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
        let link_group_id = gui.id().with("linked_plots");

        gui.vertical(|ui| {
            for (idx, pid_log) in pid_logs.iter().enumerate() {
                for (points, name, range) in pid_log.all_plots_raw() {
                    let plot_name = format!("{name} #{}", idx + 1);

                    match range {
                        ExpectedPlotRange::Percentage => {
                            if !percentage_plots.iter().any(|p| p.name == plot_name) {
                                percentage_plots.push(PlotWithName::new(points.clone(), plot_name));
                            }
                        }
                        ExpectedPlotRange::OneToOneHundred => {
                            if !to_hundreds_plots.iter().any(|p| p.name == plot_name) {
                                to_hundreds_plots
                                    .push(PlotWithName::new(points.clone(), plot_name));
                            }
                        }
                        ExpectedPlotRange::Thousands => {
                            if !to_thousands_plots.iter().any(|p| p.name == plot_name) {
                                to_thousands_plots
                                    .push(PlotWithName::new(points.clone(), plot_name));
                            }
                        }
                    }
                }
            }
            for (idx, status_log) in status_logs.iter().enumerate() {
                for (points, name, range) in status_log.all_plots_raw() {
                    let plot_name = format!("{name} #{}", idx + 1);
                    match range {
                        ExpectedPlotRange::Percentage => {
                            if !percentage_plots.iter().any(|p| p.name == plot_name) {
                                percentage_plots.push(PlotWithName::new(points.clone(), plot_name));
                            }
                        }
                        ExpectedPlotRange::OneToOneHundred => {
                            if !to_hundreds_plots.iter().any(|p| p.name == plot_name) {
                                to_hundreds_plots
                                    .push(PlotWithName::new(points.clone(), plot_name));
                            }
                        }
                        ExpectedPlotRange::Thousands => {
                            if !to_thousands_plots.iter().any(|p| p.name == plot_name) {
                                to_thousands_plots
                                    .push(PlotWithName::new(points.clone(), plot_name));
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
            let display_generator_plot = !generator_logs.is_empty();
            total_plot_count += display_generator_plot as u8;

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
                _ = percentage_plot.show(ui, |percentage_plot_ui| {
                    Self::handle_plot(percentage_plot_ui, |arg_plot_ui| {
                        for status_log in status_logs {
                            for (ts, st_change) in status_log.timestamps_with_state_changes() {
                                arg_plot_ui.text(Text::new(
                                    PlotPoint::new(*ts as f64, ((*st_change as u8) as f64) / 10.0),
                                    st_change.to_string(),
                                ));
                            }
                        }
                        skytem_logs::plot_util::plot_lines(
                            arg_plot_ui,
                            percentage_plots,
                            *line_width,
                        );
                        playback_update_plot(timer, arg_plot_ui, is_reset_pressed);
                        axis_config.handle_y_axis_lock(
                            arg_plot_ui,
                            PlotType::Percentage,
                            |plot_ui| {
                                playback_update_plot(timer, plot_ui, is_reset_pressed);
                            },
                        );
                    });
                });
            }

            if display_to_hundred_plot {
                _ = ui.separator();
                _ = to_hundred.show(ui, |to_hundred_plot_ui| {
                    Self::handle_plot(to_hundred_plot_ui, |arg_plot_ui| {
                        skytem_logs::plot_util::plot_lines(
                            arg_plot_ui,
                            to_hundreds_plots,
                            *line_width,
                        );
                        axis_config.handle_y_axis_lock(
                            arg_plot_ui,
                            PlotType::Hundreds,
                            |plot_ui| {
                                playback_update_plot(timer, plot_ui, is_reset_pressed);
                            },
                        );
                    });
                });
            }

            if display_to_thousands_plot {
                _ = ui.separator();
                _ = thousands.show(ui, |thousands_plot_ui| {
                    Self::handle_plot(thousands_plot_ui, |arg_plot_ui| {
                        skytem_logs::plot_util::plot_lines(
                            arg_plot_ui,
                            to_thousands_plots,
                            *line_width,
                        );

                        for status_log in status_logs {
                            for (ts, st_change) in status_log.timestamps_with_state_changes() {
                                arg_plot_ui.text(Text::new(
                                    PlotPoint::new(*ts as f64, (*st_change as u8) as f64),
                                    st_change.to_string(),
                                ));
                            }
                        }
                        axis_config.handle_y_axis_lock(
                            arg_plot_ui,
                            PlotType::Thousands,
                            |plot_ui| {
                                playback_update_plot(timer, plot_ui, is_reset_pressed);
                            },
                        );
                    });
                });
            }

            if display_generator_plot {
                _ = ui.separator();
                let time_formatter = |mark: GridMark, _range: &RangeInclusive<f64>| {
                    let sec = mark.value;
                    let dt = DateTime::from_timestamp(sec as i64, 0)
                        .unwrap_or_else(|| panic!("Timestamp value out of range: {sec}"));
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                };
                let x_axes = vec![AxisHints::new_x().label("Time").formatter(time_formatter)];
                let label_fmt = |_s: &str, val: &PlotPoint| {
                    let dt = DateTime::from_timestamp(val.x as i64, 0)
                        .unwrap_or_else(|| panic!("Timestamp value out of range: {}", val.x));
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

                _ = gen_log_plot.show(ui, |gen_plot_uui| {
                    Self::handle_plot(gen_plot_uui, |gen_plot_ui| {
                        let gen_log_count = generator_logs.len();
                        let mut gen_log_first_timestamp = None;
                        for (idx, gen_log) in generator_logs.iter().enumerate() {
                            if gen_log_first_timestamp.is_none() {
                                gen_log_first_timestamp =
                                    Some(gen_log.first_timestamp().unwrap_or(0.0));
                            }
                            for (raw_plot, name) in gen_log.all_plots_raw() {
                                let x_min_max_ext = skytem_logs::plot_util::extended_x_plot_bound(
                                    gen_plot_ui.plot_bounds(),
                                    0.1,
                                );
                                // Always render the first point such that the plot will always be within reasonable range
                                let filtered_points = skytem_logs::plot_util::filter_plot_points(
                                    &raw_plot,
                                    x_min_max_ext,
                                );

                                let legend_name = if gen_log_count == 1 {
                                    name
                                } else {
                                    format!("{name} #{}", idx + 1)
                                };

                                let line = Line::new(filtered_points).name(legend_name);
                                gen_plot_ui.line(line.width(*line_width));
                            }
                        }
                        axis_config.handle_y_axis_lock(
                            gen_plot_ui,
                            PlotType::Generator,
                            |plot_ui| {
                                playback_update_generator_plot(
                                    timer,
                                    plot_ui,
                                    is_reset_pressed,
                                    gen_log_first_timestamp.unwrap_or_default(),
                                );
                            },
                        );
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
}
