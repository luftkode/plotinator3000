use chrono::NaiveDateTime;
use egui::{Color32, Key, RichText, TextEdit};
use egui_phosphor::regular;

use crate::{
    app::{supported_formats::logs::parse_info::ParseInfo, WARN_ON_UNPARSED_BYTES_THRESHOLD},
    util::format_data_size,
};

use super::date_settings::LoadedLogSettings;

pub fn log_date_settings_ui(ui: &mut egui::Ui, loaded_log: &mut LoadedLogSettings) {
    let log_name_date = loaded_log.log_label();
    let check_box_text = RichText::new(if loaded_log.show_log() {
        regular::EYE
    } else {
        regular::EYE_SLASH
    });
    ui.checkbox(loaded_log.show_log_mut(), check_box_text);
    let log_button_text = RichText::new(log_name_date.clone());
    let log_button_text = if loaded_log.show_log() {
        log_button_text.strong()
    } else {
        log_button_text
    };
    let button_resp = ui.button(log_button_text);
    if button_resp.clicked() {
        loaded_log.clicked = !loaded_log.clicked;
    }
    if button_resp.hovered() {
        button_resp.on_hover_text("Click to modify log settings");
        *loaded_log.cursor_hovering_on_mut() = true;
    } else {
        *loaded_log.cursor_hovering_on_mut() = false;
    }

    ui.label(format!("{}", loaded_log.start_date().naive_utc()));

    let remove_button_text = if loaded_log.marked_for_deletion() {
        RichText::new(format!("{}", egui_phosphor::regular::TRASH)).color(Color32::RED)
    } else {
        RichText::new(format!("{}", egui_phosphor::regular::TRASH)).color(Color32::YELLOW)
    };
    let button_resp = ui.button(remove_button_text);
    if button_resp.clicked() {
        *loaded_log.marked_for_deletion_mut() = !*loaded_log.marked_for_deletion_mut();
    }
    if button_resp.hovered() {
        button_resp.on_hover_text("Remove from loaded files");
    }

    if loaded_log.tmp_date_buf.is_empty() {
        loaded_log.tmp_date_buf = loaded_log
            .start_date()
            .format("%Y-%m-%d %H:%M:%S%.f")
            .to_string();
    }
    if loaded_log.clicked {
        log_settings_window(ui, loaded_log, &log_name_date);
    }
}

fn log_settings_window(ui: &egui::Ui, settings: &mut LoadedLogSettings, log_name_date: &str) {
    // State of window bound to the 'X'-button that closes the window
    let mut open = true;
    egui::Window::new(RichText::new(log_name_date).size(20.0).strong())
        .collapsible(false)
        .movable(false)
        .open(&mut open)
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
        .show(ui.ctx(), |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(parse_info) = settings.parse_info() {
                    show_parse_info(ui, parse_info);
                }
            });
            if let Some(log_metadata) = settings.log_metadata() {
                egui::Grid::new("metadata").show(ui, |ui| {
                    for log_metadata in log_metadata {
                        log_metadata.show(ui);
                    }
                });
            }

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

fn show_parse_info(ui: &mut egui::Ui, parse_info: ParseInfo) {
    let parse_info_str = format!(
        "Parsed {parsed}/{total}",
        parsed = format_data_size(parse_info.parsed_bytes()),
        total = format_data_size(parse_info.total_bytes()),
    );
    let unparsed_text = format!(
        "({} unparsed)",
        format_data_size(parse_info.remainder_bytes())
    );
    if parse_info.remainder_bytes() > WARN_ON_UNPARSED_BYTES_THRESHOLD {
        ui.label(RichText::new("⚠").color(Color32::YELLOW));
        ui.label(parse_info_str);
        ui.label(RichText::new(unparsed_text).color(Color32::YELLOW));
    } else {
        ui.label(parse_info_str);
        ui.label(RichText::new(unparsed_text));
    }
}
