use chrono::{DateTime, NaiveDateTime, Utc};
use plot_util::PlotWithName;
use serde::{Deserialize, Serialize};
use skytem_logs::{
    generator::GeneratorLog,
    mbed_motor_control::{pid::PidLog, status::StatusLog},
};

use crate::app::PlayBackButtonEvent;
use axis_config::{AxisConfig, PlotType};
use egui::Response;
use egui_plot::{AxisHints, HPlacement, Legend, Plot, PlotPoint, Text};
use log_if::{util::ExpectedPlotRange, Plotable};
use play_state::{playback_update_plot, PlayState};
use plot_visibility_config::PlotVisibilityConfig;

mod axis_config;
mod play_state;
mod plot_ui;
mod plot_visibility_config;

#[derive(PartialEq, Deserialize, Serialize)]
pub struct LogStartDateSettings {
    pub log_id: String,
    pub original_start_date: DateTime<Utc>,
    pub start_date: DateTime<Utc>,
    pub clicked: bool,
    pub tmp_date_buf: String,
    pub err_msg: String,
    pub new_date_candidate: Option<NaiveDateTime>,
}

impl LogStartDateSettings {
    pub fn new(log_id: String, start_date: DateTime<Utc>) -> Self {
        Self {
            log_id,
            original_start_date: start_date,
            start_date,
            clicked: false,
            tmp_date_buf: String::new(),
            err_msg: String::new(),
            new_date_candidate: None,
        }
    }
}

#[allow(missing_debug_implementations)] // Legend is from egui_plot and doesn't implement debug
#[derive(PartialEq, Deserialize, Serialize)]
pub struct LogPlot {
    config: Legend,
    line_width: f32,
    axis_config: AxisConfig,
    play_state: PlayState,
    percentage_plots: Vec<PlotWithName>,
    to_hundreds_plots: Vec<PlotWithName>,
    to_thousands_plots: Vec<PlotWithName>,
    plot_visibility: PlotVisibilityConfig,
    log_start_date_settings: Vec<LogStartDateSettings>,
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
            log_start_date_settings: vec![],
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
            log_start_date_settings,
        } = self;

        let mut playback_button_event = None;

        plot_ui::show_settings_grid(
            gui,
            play_state,
            &mut playback_button_event,
            line_width,
            axis_config,
            plot_visibility,
            log_start_date_settings,
        );
        if let Some(e) = playback_button_event {
            play_state.handle_playback_button_press(e);
        };
        let is_reset_pressed = matches!(playback_button_event, Some(PlayBackButtonEvent::Reset));
        let timer = play_state.time_since_update();
        let link_group_id = gui.id().with("linked_plots");

        gui.vertical(|ui| {
            for (idx, pid_log) in pid_logs.iter().enumerate() {
                let log_id = format!("#{} {}", idx + 1, pid_log.unique_name());
                if !log_start_date_settings
                    .iter()
                    .any(|settings| *settings.log_id == log_id)
                {
                    log_start_date_settings
                        .push(LogStartDateSettings::new(log_id, pid_log.first_timestamp()));
                }
                for raw_plot in pid_log.raw_plots() {
                    let plot_name = format!("{} #{}", raw_plot.name(), idx + 1);

                    match raw_plot.expected_range() {
                        ExpectedPlotRange::Percentage => {
                            if !percentage_plots.iter().any(|p| p.name == plot_name) {
                                percentage_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
                            }
                        }
                        ExpectedPlotRange::OneToOneHundred => {
                            if !to_hundreds_plots.iter().any(|p| p.name == plot_name) {
                                to_hundreds_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
                            }
                        }
                        ExpectedPlotRange::Thousands => {
                            if !to_thousands_plots.iter().any(|p| p.name == plot_name) {
                                to_thousands_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
                            }
                        }
                    }
                }
            }
            for (idx, status_log) in status_logs.iter().enumerate() {
                let log_id = format!("#{} {}", idx + 1, status_log.unique_name());
                if !log_start_date_settings
                    .iter()
                    .any(|settings| *settings.log_id == log_id)
                {
                    log_start_date_settings.push(LogStartDateSettings::new(
                        log_id,
                        status_log.first_timestamp(),
                    ));
                }
                for raw_plot in status_log.raw_plots() {
                    let plot_name = format!("{} #{}", raw_plot.name(), idx + 1);
                    match raw_plot.expected_range() {
                        ExpectedPlotRange::Percentage => {
                            if !percentage_plots.iter().any(|p| p.name == plot_name) {
                                percentage_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
                            }
                        }
                        ExpectedPlotRange::OneToOneHundred => {
                            if !to_hundreds_plots.iter().any(|p| p.name == plot_name) {
                                to_hundreds_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
                            }
                        }
                        ExpectedPlotRange::Thousands => {
                            if !to_thousands_plots.iter().any(|p| p.name == plot_name) {
                                to_thousands_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
                            }
                        }
                    }
                }
            }
            for (idx, gen_log) in generator_logs.iter().enumerate() {
                let log_id = format!("#{} {}", idx + 1, gen_log.unique_name());
                if !log_start_date_settings
                    .iter()
                    .any(|settings| *settings.log_id == log_id)
                {
                    log_start_date_settings
                        .push(LogStartDateSettings::new(log_id, gen_log.first_timestamp()));
                }
                for raw_plot in gen_log.raw_plots() {
                    let plot_name = format!("{}, #{}", raw_plot.name(), idx + 1);
                    match raw_plot.expected_range() {
                        ExpectedPlotRange::Percentage => {
                            if !percentage_plots.iter().any(|p| p.name == plot_name) {
                                percentage_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
                            }
                        }
                        ExpectedPlotRange::OneToOneHundred => {
                            if !to_hundreds_plots.iter().any(|p| p.name == plot_name) {
                                to_hundreds_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
                            }
                        }
                        ExpectedPlotRange::Thousands => {
                            if !to_thousands_plots.iter().any(|p| p.name == plot_name) {
                                to_thousands_plots
                                    .push(PlotWithName::new(raw_plot.points().to_vec(), plot_name));
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

            let plot_height = ui.available_height() / (total_plot_count as f32);

            let x_axes = vec![AxisHints::new_x()
                .label("Time")
                .formatter(crate::util::format_time)];

            let create_plot = |name: &str| {
                Plot::new(name)
                    .legend(config.clone())
                    .height(plot_height)
                    .show_axes(axis_config.show_axes())
                    .y_axis_position(HPlacement::Right)
                    .include_y(0.0)
                    .custom_x_axes(x_axes.clone())
                    .label_formatter(crate::util::format_label_ns)
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
                                    PlotPoint::new(*ts, ((*st_change as u8) as f64) / 10.0),
                                    st_change.to_string(),
                                ));
                            }
                        }
                        plot_util::plot_lines(arg_plot_ui, percentage_plots, *line_width);
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
                        plot_util::plot_lines(arg_plot_ui, to_hundreds_plots, *line_width);
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
                        plot_util::plot_lines(arg_plot_ui, to_thousands_plots, *line_width);

                        for status_log in status_logs {
                            for (ts, st_change) in status_log.timestamps_with_state_changes() {
                                arg_plot_ui.text(Text::new(
                                    PlotPoint::new(*ts, (*st_change as u8) as f64),
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
