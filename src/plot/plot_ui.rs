use chrono::{DateTime, NaiveDateTime, Utc};
use egui::{Color32, RichText, TextEdit};
use egui_phosphor::regular;

use crate::app::PlayBackButtonEvent;

use super::{
    axis_config::AxisConfig, play_state::PlayState, plot_visibility_config::PlotVisibilityConfig,
    LogStartDateSettings,
};

pub fn show_settings_grid(
    gui: &mut egui::Ui,
    play_state: &PlayState,
    playback_button_event: &mut Option<PlayBackButtonEvent>,
    line_width: &mut f32,
    axis_cfg: &mut AxisConfig,
    plot_visibility_cfg: &mut PlotVisibilityConfig,
    log_start_date_settings: &mut [LogStartDateSettings],
) {
    _ = egui::Grid::new("settings").show(gui, |arg_ui| {
        _ = arg_ui.label("Line width");
        _ = arg_ui.add(
            egui::DragValue::new(line_width)
                .speed(0.02)
                .range(0.5..=20.0),
        );
        _ = arg_ui.horizontal_top(|ui| {
            axis_cfg.toggle_axis_cfg_ui(ui);
            _ = ui.label("|");
            plot_visibility_cfg.toggle_visibility_ui(ui);
        });

        _ = arg_ui.horizontal_centered(|ui| {
            _ = ui.label("| ");
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

            _ = ui.label(RichText::new(play_state.formatted_time()));
            _ = ui.label(" |");
        });
        for settings in log_start_date_settings {
            let log_name_date = format!("{} [{}]", settings.log_id, settings.start_date);
            if arg_ui.button(log_name_date.clone()).clicked() {
                settings.clicked = !settings.clicked;
            }
            if settings.tmp_date_buf.is_empty() {
                settings.tmp_date_buf = settings
                    .start_date
                    .format("%Y-%m-%d %H:%M:%S%.f")
                    .to_string();
            }
            if settings.clicked {
                egui::Window::new(RichText::new(log_name_date).size(20.0).strong())
                    .collapsible(false)
                    .movable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                    .show(arg_ui.ctx(), |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label("Modify the start date to offset the plots of this log");
                            ui.label(format!("original date: {}", settings.original_start_date));
                            ui.label(RichText::new("YYYY-mm-dd HH:MM:SS.ms").strong());
                            let response = ui.add(TextEdit::singleline(&mut settings.tmp_date_buf));
                            if response.changed() {
                                log::debug!("Changed to {}", settings.tmp_date_buf);
                                match NaiveDateTime::parse_from_str(
                                    &settings.tmp_date_buf,
                                    "%Y-%m-%d %H:%M:%S%.f",
                                ) {
                                    Ok(new_dt) => {
                                        settings.err_msg.clear();
                                        settings.new_date_candidate = Some(new_dt);
                                    }
                                    Err(e) => {
                                        _ = {
                                            settings.err_msg = format!("⚠ {e} ⚠");
                                        }
                                    }
                                };
                            }
                            if settings.err_msg.is_empty() {
                                if let Some(new_date) = settings.new_date_candidate {
                                    if ui.button("Apply").clicked() {
                                        settings.start_date = new_date.and_utc();
                                        log::info!("New date: {}", settings.start_date);
                                    }
                                }
                            } else {
                                ui.label(settings.err_msg.clone());
                            }
                            if ui.button("Cancel").clicked() {
                                settings.clicked = false;
                            }
                        })
                    });
            }
        }
        arg_ui.end_row();
    });
}
