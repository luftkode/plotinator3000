use chrono::NaiveDateTime;
use egui::{Key, RichText, TextEdit};
use egui_phosphor::regular;

use super::date_settings::LogStartDateSettings;

pub fn log_date_settings_ui(ui: &mut egui::Ui, settings: &mut LogStartDateSettings) {
    let log_name_date = settings.log_label();
    let check_box_text = RichText::new(if settings.show_log() {
        regular::EYE
    } else {
        regular::EYE_SLASH
    });
    ui.checkbox(settings.show_log_mut(), check_box_text);
    let log_button_text = RichText::new(log_name_date.clone());
    let log_button_text = if settings.show_log() {
        log_button_text.strong()
    } else {
        log_button_text
    };
    let button_resp = ui.button(log_button_text);
    if button_resp.clicked() {
        settings.clicked = !settings.clicked;
    }
    if button_resp.hovered() {
        button_resp.on_hover_text("Click to modify log settings");
    }

    if settings.tmp_date_buf.is_empty() {
        settings.tmp_date_buf = settings
            .start_date()
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
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
        .show(ui.ctx(), |ui| {
            ui.vertical_centered(|ui| {
                ui.label("Modify the start date to offset the plots of this log");
                ui.label(format!("original date: {}", settings.original_start_date));
                ui.label(RichText::new("YYYY-mm-dd HH:MM:SS.ms").strong());
                let date_txt_input_resp = ui.add(TextEdit::singleline(&mut settings.tmp_date_buf));
                date_txt_input_resp.request_focus();
                if date_txt_input_resp.changed() {
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
                            settings.new_start_date(new_date.and_utc());
                            settings.date_changed = true;
                            log::info!("New date: {}", settings.start_date());
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
