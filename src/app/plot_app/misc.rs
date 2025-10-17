use egui_phosphor::regular::{self, TRASH};
use plotinator_file_io::loaded_files::LoadedFiles;
use plotinator_log_if::prelude::Plotable as _;
use plotinator_plot_ui::WARN_ON_UNPARSED_BYTES_THRESHOLD;
use std::time::Duration;

use egui::{Color32, RichText, TextStyle, ThemePreference};
use egui_notify::Toasts;
use plotinator_strfmt::format_data_size;
use plotinator_supported_formats::SupportedFormat;

use crate::PlotApp;

pub(super) fn show_theme_toggle_buttons(ui: &mut egui::Ui) {
    let mut theme_preference = ui.ctx().options(|opt| opt.theme_preference);

    ui.horizontal(|ui| {
        ui.selectable_value(&mut theme_preference, ThemePreference::Light, "☀");
        ui.selectable_value(&mut theme_preference, ThemePreference::Dark, "🌙 ");
        ui.selectable_value(&mut theme_preference, ThemePreference::System, "💻");
    });

    ui.ctx().set_theme(theme_preference);
}

/// Displays a toasts notification if logs are added with the names of all added logs
pub(super) fn notify_if_logs_added(toasts: &mut Toasts, logs: &[SupportedFormat]) {
    if !logs.is_empty() {
        let mut log_names_str = String::new();
        for l in logs {
            log_names_str.push('\n');
            log_names_str.push('\t');
            log_names_str.push_str(l.descriptive_name());
        }
        toasts
            .info(format!(
                "{} log{} added{log_names_str}",
                logs.len(),
                if logs.len() == 1 { "" } else { "s" }
            ))
            .duration(Some(Duration::from_secs(2)));
        for l in logs {
            if let Some(parse_info) = l.parse_info() {
                log::debug!(
                    "Unparsed bytes for {log_name}: {remainder}",
                    remainder = parse_info.remainder_bytes(),
                    log_name = l.descriptive_name()
                );
                if parse_info.remainder_bytes() > WARN_ON_UNPARSED_BYTES_THRESHOLD {
                    toasts
                        .warning(format!(
                    "Could only parse {parsed}/{total} for {log_name}\n{remainder} remain unparsed",
                    parsed = format_data_size(parse_info.parsed_bytes()),
                    total = format_data_size(parse_info.total_bytes()),
                    log_name = l.descriptive_name(),
                    remainder = format_data_size(parse_info.remainder_bytes())
                ))
                        .duration(Some(Duration::from_secs(30)));
                }
            }
        }
    }
}

pub(super) fn configure_text_styles(ctx: &egui::Context, font_size: f32) {
    let mut style = (*ctx.style()).clone();
    for font_id in style.text_styles.values_mut() {
        font_id.size = font_size;
    }
    ctx.set_style(style);
}

pub(super) fn collapsible_instructions(ui: &mut egui::Ui) {
    ui.collapsing("Instructions", |ui| {
        ui.label("Pan: Drag, or scroll (+ shift = horizontal).");
        ui.label("Box zooming: Right click + drag.");
        if cfg!(target_os = "macos") {
            ui.label("X-axis zoom: CTRL/⌘ + scroll.");
            ui.label("Y-axis zoom: CTRL/⌘ + ALT + scroll.");
        } else {
            ui.label("X-axis zoom: CTRL + scroll.");
            ui.label("Y-axis zoom: CTRL + ALT + scroll.");
        }
        ui.label("Reset view: double-click.");
    });
}

pub(super) fn show_homepage_link(ui: &mut egui::Ui) {
    ui.add(egui::Hyperlink::from_label_and_url(
        "Homepage",
        "https://github.com/luftkode/plotinator3000",
    ));
}

pub(super) fn show_font_size_drag_value(ui: &mut egui::Ui, ctx: &egui::Context, app: &mut PlotApp) {
    ui.label(RichText::new(regular::TEXT_T));
    if ui
        .add(
            egui::DragValue::new(&mut app.font_size)
                .speed(0.1)
                .range(8.0..=32.0)
                .suffix("px"),
        )
        .changed()
    {
        configure_text_styles(ctx, app.font_size);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn not_wasm_show_download_button(ui: &mut egui::Ui, app: &mut PlotApp) {
    if ui
        .button(RichText::new(format!(
            "{icon} Download",
            icon = egui_phosphor::regular::DOWNLOAD_SIMPLE
        )))
        .clicked()
    {
        app.download_ui.show = true;
    }
}

pub(super) fn show_app_reset_button(ui: &mut egui::Ui, app: &mut PlotApp) {
    if ui.button(RichText::new(format!("{TRASH} Reset"))).clicked() {
        if app.plot.plot_count() == 0 {
            app.toasts
                .warning("No loaded plots...")
                .duration(Some(std::time::Duration::from_secs(3)));
        } else {
            app.toasts
                .info("All loaded logs removed...")
                .duration(Some(std::time::Duration::from_secs(3)));
        }
        app.loaded_files = LoadedFiles::default();
        app.plot = plotinator_plot_ui::LogPlotUi::default();

        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
        {
            app.mqtt.reset();
            app.map_commander.reset_map_data();
        }
    }
}

pub(super) fn show_error(ui: &egui::Ui, app: &mut PlotApp) {
    if let Some(error) = app.error_message.clone() {
        egui::Window::new(RichText::new("⚠").size(40.0).color(Color32::RED))
            .auto_sized()
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new(&error).text_style(TextStyle::Body).strong());
                    ui.add_space(20.0);

                    let button_text = RichText::new("OK")
                        .text_style(TextStyle::Heading)
                        .size(18.0)
                        .strong();

                    let button_size = egui::Vec2::new(80.0, 40.0);
                    if ui
                        .add_sized(button_size, egui::Button::new(button_text))
                        .on_hover_text("Click to dismiss the error")
                        .clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        app.error_message = None;
                    }
                    ui.add_space(20.0);
                });
            });
    }
}

pub(super) fn show_warn_on_debug_build(ui: &mut egui::Ui) {
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        egui::warn_if_debug_build(ui);
    });
}
