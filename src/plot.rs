use date_settings::LogStartDateSettings;
use log_if::plotable::Plotable;
use plot_util::PlotData;
use serde::{Deserialize, Serialize};

use crate::app::PlayBackButtonEvent;
use axis_config::{AxisConfig, PlotType};
use egui::{Response, RichText};
use egui_plot::{AxisHints, HPlacement, Legend, Plot, PlotPoint};
use play_state::{playback_update_plot, PlayState};
use plot_visibility_config::PlotVisibilityConfig;

mod axis_config;
mod date_settings;
mod play_state;
mod plot_ui;
mod plot_visibility_config;
mod util;

#[allow(missing_debug_implementations)] // Legend is from egui_plot and doesn't implement debug
#[derive(PartialEq, Deserialize, Serialize)]
pub struct LogPlotUi {
    config: Legend,
    line_width: f32,
    axis_config: AxisConfig,
    play_state: PlayState,
    plot_data_percentage: PlotData,
    plot_data_to_hundreds: PlotData,
    plot_data_thousands: PlotData,
    plot_visibility: PlotVisibilityConfig,
    log_start_date_settings: Vec<LogStartDateSettings>,
    x_min_max: Option<(f64, f64)>,
    // Various info about the plot is invalidated if this is true (so it needs to be recalculated)
    invalidate_plot: bool,
}

impl Default for LogPlotUi {
    fn default() -> Self {
        Self {
            config: Default::default(),
            line_width: 1.5,
            axis_config: Default::default(),
            play_state: PlayState::default(),
            plot_data_percentage: PlotData::default(),
            plot_data_to_hundreds: PlotData::default(),
            plot_data_thousands: PlotData::default(),
            plot_visibility: PlotVisibilityConfig::default(),
            log_start_date_settings: vec![],
            x_min_max: None,
            invalidate_plot: false,
        }
    }
}

impl LogPlotUi {
    pub fn formatted_playback_time(&self) -> String {
        self.play_state.formatted_time()
    }
    pub fn is_playing(&self) -> bool {
        self.play_state.is_playing()
    }

    // TODO: Fix this lint
    #[allow(clippy::too_many_lines)]
    pub fn ui(&mut self, gui: &mut egui::Ui, logs: &[Box<dyn Plotable>]) -> Response {
        let Self {
            config,
            line_width,
            axis_config,
            play_state,
            plot_data_percentage,
            plot_data_to_hundreds,
            plot_data_thousands,
            plot_visibility,
            log_start_date_settings,
            x_min_max,
            invalidate_plot,
        } = self;

        // Various stored knowledge about the plot needs to be reset and recalculated if the plot is invalidated
        if *invalidate_plot {
            *x_min_max = None;
            *invalidate_plot = false;
        }

        util::calc_all_plot_x_min_max(
            plot_data_percentage.plots(),
            plot_data_to_hundreds.plots(),
            plot_data_thousands.plots(),
            x_min_max,
        );

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
            for log in logs {
                util::add_plot_data_to_plot_collections(
                    log_start_date_settings,
                    plot_data_percentage,
                    plot_data_to_hundreds,
                    plot_data_thousands,
                    log.as_ref(),
                );
            }

            for settings in log_start_date_settings {
                date_settings::update_plot_dates(
                    invalidate_plot,
                    plot_data_percentage,
                    plot_data_to_hundreds,
                    plot_data_thousands,
                    settings,
                );
            }

            // Calculate the number of plots to display
            let mut total_plot_count: u8 = 0;
            let display_percentage_plot =
                plot_visibility.should_display_percentage(plot_data_percentage.plots());
            total_plot_count += display_percentage_plot as u8;
            let display_to_hundred_plot =
                plot_visibility.should_display_to_hundreds(plot_data_to_hundreds.plots());
            total_plot_count += display_to_hundred_plot as u8;
            let display_to_thousands_plot =
                plot_visibility.should_display_to_thousands(plot_data_thousands.plots());
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
                    .show_grid(axis_config.show_grid())
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
                percentage_plot.show(ui, |percentage_plot_ui| {
                    Self::handle_plot(percentage_plot_ui, |plot_ui| {
                        plot_util::plot_lines(plot_ui, plot_data_percentage.plots(), *line_width);
                        for plot_labels in plot_data_percentage.plot_labels() {
                            for (label_point, label_txt) in plot_labels.label_points() {
                                let point = PlotPoint::new(label_point[0], label_point[1]);
                                let txt = RichText::new(label_txt.as_str()).size(10.0);
                                let txt = egui_plot::Text::new(point, txt);
                                plot_ui.text(txt);
                            }
                        }
                        playback_update_plot(
                            timer,
                            plot_ui,
                            is_reset_pressed,
                            x_min_max.unwrap_or_default().0,
                        );
                        axis_config.handle_y_axis_lock(plot_ui, PlotType::Percentage, |plot_ui| {
                            playback_update_plot(
                                timer,
                                plot_ui,
                                is_reset_pressed,
                                x_min_max.unwrap_or_default().0,
                            );
                        });
                    });
                });
            }

            if display_to_hundred_plot {
                ui.separator();
                to_hundred.show(ui, |to_hundred_plot_ui| {
                    Self::handle_plot(to_hundred_plot_ui, |plot_ui| {
                        plot_util::plot_lines(plot_ui, plot_data_to_hundreds.plots(), *line_width);
                        for plot_labels in plot_data_to_hundreds.plot_labels() {
                            for (label_point, label_txt) in plot_labels.label_points() {
                                let point = PlotPoint::new(label_point[0], label_point[1]);
                                let txt = RichText::new(label_txt.as_str()).size(10.0);
                                let txt = egui_plot::Text::new(point, txt);
                                plot_ui.text(txt);
                            }
                        }

                        axis_config.handle_y_axis_lock(plot_ui, PlotType::Hundreds, |plot_ui| {
                            playback_update_plot(
                                timer,
                                plot_ui,
                                is_reset_pressed,
                                x_min_max.unwrap_or_default().0,
                            );
                        });
                    });
                });
            }

            if display_to_thousands_plot {
                ui.separator();
                thousands.show(ui, |thousands_plot_ui| {
                    Self::handle_plot(thousands_plot_ui, |plot_ui| {
                        plot_util::plot_lines(plot_ui, plot_data_thousands.plots(), *line_width);
                        for plot_labels in plot_data_thousands.plot_labels() {
                            for (label_point, label_txt) in plot_labels.label_points() {
                                let point = PlotPoint::new(label_point[0], label_point[1]);
                                let txt = RichText::new(label_txt.as_str()).size(10.0);
                                let txt = egui_plot::Text::new(point, txt);
                                plot_ui.text(txt);
                            }
                        }
                        axis_config.handle_y_axis_lock(plot_ui, PlotType::Thousands, |plot_ui| {
                            playback_update_plot(
                                timer,
                                plot_ui,
                                is_reset_pressed,
                                x_min_max.unwrap_or_default().0,
                            );
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
}
