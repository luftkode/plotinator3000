use chrono::NaiveDateTime;
use egui::{Color32, Key, RichText, TextEdit};
use egui_phosphor::regular;

use crate::app::PlayBackButtonEvent;

use super::{
    axis_config::AxisConfig, play_state::PlayState, plot_visibility_config::PlotVisibilityConfig,
    LogStartDateSettings,
};

pub fn show_settings_grid(
    ui: &mut egui::Ui,
    play_state: &PlayState,
    playback_button_event: &mut Option<PlayBackButtonEvent>,
    line_width: &mut f32,
    axis_cfg: &mut AxisConfig,
    plot_visibility_cfg: &mut PlotVisibilityConfig,
    log_start_date_settings: &mut [LogStartDateSettings],
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
        for settings in log_start_date_settings {
            log_date_settings_ui(ui, settings);
        }

        ui.end_row();
    });
}

pub fn log_date_settings_ui(ui: &mut egui::Ui, settings: &mut LogStartDateSettings) {
    ui.end_row();
    let log_name_date = format!("{} [{}]", settings.log_id, settings.start_date);
    let button_resp = ui.button(log_name_date.clone());
    if button_resp.clicked() {
        settings.clicked = !settings.clicked;
    }
    button_resp.on_hover_text("Click to modify log settings");

    if settings.tmp_date_buf.is_empty() {
        settings.tmp_date_buf = settings
            .start_date
            .format("%Y-%m-%d %H:%M:%S%.f")
            .to_string();
    }
    if settings.clicked {
        log_settings_window(ui, settings, &log_name_date);
    }
}

fn log_settings_window(ui: &egui::Ui, settings: &mut LogStartDateSettings, log_name_date: &str) {
    // State of window bound to the 'X'-button that closes the window
    let mut open = true;
    egui::Window::new(RichText::new(log_name_date).size(20.0).strong())
        .collapsible(false)
        .movable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ui.ctx(), |ui| {
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
                            settings.err_msg = format!("⚠ {e} ⚠");
                        }
                    };
                }
                if settings.err_msg.is_empty() {
                    if let Some(new_date) = settings.new_date_candidate {
                        if ui.button("Apply").clicked() || ui.input(|i| i.key_pressed(Key::Enter)) {
                            settings.start_date = new_date.and_utc();
                            settings.date_changed = true;
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

    if !open || ui.ctx().input(|i| i.key_pressed(Key::Escape)) {
        settings.clicked = false;
    }
}
