use egui::{Color32, Key, RichText};
use egui_phosphor::regular;
use loaded_logs::LoadedLogsUi;

use crate::app::PlayBackButtonEvent;

use super::{axis_config::AxisConfig, play_state::PlayState, plot_settings::PlotSettings};

pub mod loaded_logs;

// filter settings should be refactored out to be a standalone thing, maybe together with loaded_logs_ui
#[allow(clippy::too_many_arguments)]
pub fn show_settings_grid(
    ui: &mut egui::Ui,
    play_state: &PlayState,
    playback_button_event: &mut Option<PlayBackButtonEvent>,
    line_width: &mut f32,
    axis_cfg: &mut AxisConfig,
    plot_settings: &mut PlotSettings,
    mut loaded_logs_ui: LoadedLogsUi<'_>,
    show_filter_settings: &mut bool,
    plot_names_show: &mut Vec<(String, bool)>,
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
            plot_settings.show(ui);
        });

        ui.horizontal_centered(|ui| {
            ui.label("| ");
            // Reset button
            let reset_text = RichText::new(egui_phosphor::regular::REWIND);
            if ui.button(reset_text).clicked() {
                *playback_button_event = Some(PlayBackButtonEvent::Reset);
            }
            let playpause_text = if play_state.is_playing() {
                RichText::new(regular::PAUSE).color(Color32::YELLOW)
            } else {
                RichText::new(regular::PLAY).color(Color32::GREEN)
            };
            if ui.button(playpause_text).clicked() {
                *playback_button_event = Some(PlayBackButtonEvent::PlayPause);
            }

            ui.label(RichText::new(play_state.formatted_time()));
            ui.label(" |");
        });

        ui.end_row();
    });
    ui.horizontal_wrapped(|ui| {
        let filter_settings_text = RichText::new(format!(
            "{} Filter {}",
            regular::FUNNEL,
            regular::CHART_LINE
        ));
        ui.toggle_value(show_filter_settings, filter_settings_text.text());
        if *show_filter_settings {
            egui::Window::new(filter_settings_text)
                .open(show_filter_settings)
                .show(ui.ctx(), |ui| {
                    let mut enable_all = false;
                    let mut disable_all = false;
                    egui::Grid::new("global_filter_settings").show(ui, |ui| {
                        if ui
                            .button(RichText::new("Show all").strong().heading())
                            .clicked()
                        {
                            enable_all = true;
                        }
                        if ui
                            .button(RichText::new("Hide all").strong().heading())
                            .clicked()
                        {
                            disable_all = true;
                        }
                    });
                    if enable_all {
                        for (_, show) in &mut *plot_names_show {
                            *show = true;
                        }
                    } else if disable_all {
                        for (_, show) in &mut *plot_names_show {
                            *show = false;
                        }
                    }

                    for (pname, show) in plot_names_show {
                        ui.toggle_value(show, pname.as_str());
                    }
                });
            if ui.ctx().input(|i| i.key_pressed(Key::Escape)) {
                *show_filter_settings = false;
            }
        }
        loaded_logs_ui.show(ui);
    });
}
