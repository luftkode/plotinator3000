use egui::{Color32, Key, RichText};
use egui_phosphor::regular;
use egui_plot::PlotBounds;
use plotinator_strfmt::format_data_size;

use crate::WARN_ON_UNPARSED_BYTES_THRESHOLD;

use super::date_settings::LoadedLogSettings;

fn ui_button_toggle_visibility(ui: &mut egui::Ui, loaded_log: &mut LoadedLogSettings) {
    let show_toggle_txt = if loaded_log.show_log() {
        RichText::new(regular::EYE).color(Color32::GREEN)
    } else {
        RichText::new(regular::EYE_SLASH)
    };

    let show_toggle = ui.button(show_toggle_txt);
    if show_toggle.clicked() {
        *loaded_log.show_log_mut() = !*loaded_log.show_log_mut();
    }
    if show_toggle.hovered() {
        *loaded_log.cursor_hovering_on_mut() = true;
    }
}

fn ui_button_remove_toggle(ui: &mut egui::Ui, loaded_log: &mut LoadedLogSettings) {
    let remove_button_text = if loaded_log.marked_for_deletion() {
        RichText::new(regular::TRASH).color(Color32::RED)
    } else {
        RichText::new(regular::TRASH).color(Color32::YELLOW)
    };
    let button_resp = ui.button(remove_button_text);
    if button_resp.clicked() {
        *loaded_log.marked_for_deletion_mut() = !*loaded_log.marked_for_deletion_mut();
    }
    if button_resp.hovered() {
        button_resp.on_hover_text("Remove from loaded files");
        *loaded_log.cursor_hovering_on_mut() = true;
    }
}

fn ui_button_open_log(ui: &mut egui::Ui, loaded_log: &mut LoadedLogSettings) {
    let log_button_text = RichText::new(loaded_log.log_label());
    let log_button_text = if loaded_log.show_log() {
        log_button_text.strong()
    } else {
        log_button_text
    };
    let ui_log_button = ui.button(log_button_text);
    if ui_log_button.clicked() {
        loaded_log.toggle_clicked();
    }
    if ui_log_button.hovered() {
        ui_log_button.on_hover_text("Click to modify log settings");
        *loaded_log.cursor_hovering_on_mut() = true;
    }

    let ui_date_label = ui.label(loaded_log.starte_date_formatted());
    if ui_date_label.hovered() {
        *loaded_log.cursor_hovering_on_mut() = true;
    }
}

pub fn show_log_settings_ui(
    ui: &mut egui::Ui,
    loaded_log: &mut LoadedLogSettings,
    selected_box: Option<PlotBounds>,
) {
    ui_button_open_log(ui, loaded_log);
    ui.label("");
    ui.horizontal(|ui| {
        ui_button_toggle_visibility(ui, loaded_log);
        ui_button_remove_toggle(ui, loaded_log);
    });
    if loaded_log.clicked() {
        log_settings_window(ui, loaded_log, &loaded_log.log_label(), selected_box);
    }
}

fn log_settings_window(
    ui: &egui::Ui,
    settings: &mut LoadedLogSettings,
    log_name_date: &str,
    selected_box: Option<PlotBounds>,
) {
    // State of window bound to the 'X'-button that closes the window
    let mut open = true;
    egui::Window::new(RichText::new(log_name_date).size(20.0).strong())
        .collapsible(false)
        .movable(false)
        .open(&mut open)
        .default_size([450.0, 400.0]) // Set a reasonable default size
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::ZERO)
        .show(ui.ctx(), |ui| {
            // Create a panel at the bottom for the date controls.
            // This reserves space for the controls first.
            egui::TopBottomPanel::bottom("date_controls_panel")
                .resizable(false)
                .show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.separator();
                        ui.label("Modify the start date to offset the plots of this log");
                        ui.label(format!("original date: {}", settings.original_start_date));
                        ui.label(RichText::new("YYYY-mm-dd HH:MM:SS.ms").strong());
                        let date_txt_input_resp = settings.start_date_editor.show(ui);
                        if !settings.log_points_cutter.clicked {
                            date_txt_input_resp.request_focus();
                        }

                        if let Some(new_date) = settings.start_date_editor.current()
                            && (ui.button("Apply").clicked()
                                || ui.input(|i| i.key_pressed(Key::Enter)))
                        {
                            settings.new_start_date(new_date);
                            log::info!("New date: {}", settings.start_date());
                        }
                        if ui.button("Cancel").clicked() {
                            *settings.clicked_mut() = false;
                        }
                    });
                });

            ui.horizontal_wrapped(|ui| {
                if let Some(parse_info) = settings.parse_info() {
                    show_parse_info(ui, parse_info);
                }
            });

            if ui.button("Cut Points").clicked() {
                settings.log_points_cutter.clicked = true;
            }
            if let Some(log_metadata) = settings.log_metadata() {
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("metadata").show(ui, |ui| {
                            for item in log_metadata {
                                item.show(ui);
                            }
                        });
                    });
            }
        });

    if settings.log_points_cutter.clicked {
        settings
            .log_points_cutter
            .show(ui, log_name_date, selected_box);
    }

    if !open || ui.ctx().input(|i| i.key_pressed(Key::Escape)) {
        *settings.clicked_mut() = false;
        settings.log_points_cutter.clicked = false;
    }
}

fn show_parse_info(ui: &mut egui::Ui, parse_info: plotinator_supported_formats::ParseInfo) {
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
