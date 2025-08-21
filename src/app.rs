use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

use crate::{
    app::download::{ENDPOINT_DOWNLOAD_LATEST, ENDPOINT_DOWNLOAD_TODAY},
    plot::LogPlotUi,
    util::format_data_size,
};
use dropped_files::handle_dropped_files;
use egui::{Color32, Hyperlink, RichText, TextStyle, ThemePreference, UiKind};
use egui_notify::Toasts;
use egui_phosphor::regular;
use plotinator_log_if::prelude::Plotable as _;

use file_dialog as fd;
use loaded_files::LoadedFiles;
use plotinator_supported_formats::SupportedFormat;

pub(crate) mod custom_files;
mod dropped_files;
mod file_dialog;
pub mod loaded_files;
mod util;

mod download;

/// if a log is loaded from content that exceeds this many unparsed bytes:
/// - Show a toasts warning notification
/// - Show warnings in the UI when viewing parse info for the loaded log
pub const WARN_ON_UNPARSED_BYTES_THRESHOLD: usize = 128;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    // Set on the very first frame of starting the app
    #[serde(skip)]
    first_frame: bool,
    #[serde(skip)]
    toasts: Toasts,

    loaded_files: LoadedFiles,
    plot: LogPlotUi,
    font_size: f32,
    font_size_init: bool,
    error_message: Option<String>,

    // Download configuration
    download_host: String,
    download_port: String,

    #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
    #[serde(skip)]
    mqtt: crate::mqtt::Mqtt,

    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    web_file_dialog: fd::web::WebFileDialog,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    native_file_dialog: fd::native::NativeFileDialog,

    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    keep_repainting: bool,

    #[serde(skip)]
    download_manager: DownloadManager,
    show_download_window: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            first_frame: true,
            toasts: Toasts::default(),
            loaded_files: LoadedFiles::default(),
            plot: LogPlotUi::default(),
            font_size: Self::DEFAULT_FONT_SIZE,
            font_size_init: false,
            error_message: None,
            download_host: "localhost".to_string(),
            download_port: "8080".to_string(),

            #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
            mqtt: crate::mqtt::Mqtt::default(),

            #[cfg(target_arch = "wasm32")]
            web_file_dialog: fd::web::WebFileDialog::default(),

            #[cfg(not(target_arch = "wasm32"))]
            native_file_dialog: fd::native::NativeFileDialog::default(),

            #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
            keep_repainting: true,

            download_manager: DownloadManager::new(),
            show_download_window: false,
        }
    }
}

impl App {
    const DEFAULT_FONT_SIZE: f32 = 16.0;

    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        cc.egui_ctx.set_fonts(fonts);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.font_size_init = false;
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_picked_files();
        // Poll download messages
        for msg in self.download_manager.poll() {
            match msg {
                DownloadMessage::Success(filename) => {
                    self.toasts
                        .success(format!("Downloaded: {}", filename))
                        .duration(Some(Duration::from_secs(5)));
                }
                DownloadMessage::Error(err) => {
                    self.toasts
                        .error(format!("Download failed: {}", err))
                        .duration(Some(Duration::from_secs(10)));
                }
                DownloadMessage::Progress {
                    downloaded_bytes,
                    total_bytes,
                } => {
                    self.download_manager
                        .update_progress(downloaded_bytes, total_bytes);
                }
                DownloadMessage::Finished => {}
            }
            ctx.request_repaint();
        }

        if !self.font_size_init {
            configure_text_styles(ctx, self.font_size);
        }

        show_top_panel(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            notify_if_logs_added(&mut self.toasts, self.loaded_files.loaded());
            self.plot.ui(
                ui,
                &mut self.first_frame,
                &self.loaded_files.take_loaded_files(),
                &mut self.toasts,
                #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
                &mut self.mqtt,
            );

            if self.plot.plot_count() == 0 {
                // Display the message when plots are shown
                util::draw_empty_state(ui);
            }

            match handle_dropped_files(ctx, &mut self.loaded_files) {
                Ok(Some(new_plot_ui_state)) => self.load_new_plot_ui_state(new_plot_ui_state),
                Err(e) => self.error_message = Some(e.to_string()),
                Ok(None) => (),
            }

            self.show_error(ui);
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
        show_download_window(self, ctx);

        self.toasts.show(ctx);
    }
}

impl App {
    fn load_new_plot_ui_state(&mut self, new: Box<LogPlotUi>) {
        self.first_frame = true; // Necessary to reset some caching
        self.plot = *new;
    }

    fn show_error(&mut self, ui: &egui::Ui) {
        if let Some(error) = self.error_message.clone() {
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
                            self.error_message = None;
                        }
                        ui.add_space(20.0);
                    });
                });
        }
    }

    fn poll_picked_files(&mut self) {
        #[cfg(target_arch = "wasm32")]
        match self
            .web_file_dialog
            .poll_received_files(&mut self.loaded_files)
        {
            Ok(Some(new_plot_ui_state)) => self.load_new_plot_ui_state(new_plot_ui_state),
            Err(e) => self.error_message = Some(e.to_string()),
            Ok(None) => (),
        }
        #[cfg(not(target_arch = "wasm32"))]
        match self
            .native_file_dialog
            .parse_picked_files(&mut self.loaded_files)
        {
            Ok(Some(new_plot_ui_state)) => self.load_new_plot_ui_state(new_plot_ui_state),
            Err(e) => self.error_message = Some(e.to_string()),
            Ok(None) => (),
        }
    }
}

fn collapsible_instructions(ui: &mut egui::Ui) {
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

fn show_top_panel(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            if ui
                .button(RichText::new(format!(
                    "{icon} Reset",
                    icon = egui_phosphor::regular::TRASH
                )))
                .clicked()
            {
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
                app.plot = LogPlotUi::default();

                #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
                app.mqtt.reset();
            }
            if ui
                .button(RichText::new(format!(
                    "{icon} Open File",
                    icon = egui_phosphor::regular::FOLDER_OPEN
                )))
                .clicked()
            {
                #[cfg(target_arch = "wasm32")]
                app.web_file_dialog.open(ctx.clone());
                #[cfg(not(target_arch = "wasm32"))]
                app.native_file_dialog.open();
            }

            ui.menu_button(
                RichText::new(format!(
                    "{icon} Save",
                    icon = egui_phosphor::regular::FLOPPY_DISK
                )),
                |ui| {
                    // Option to export the entire UI state for later restoration
                    if ui.button("Plot UI State").clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        file_dialog::native::NativeFileDialog::save_plot_ui(&app.plot);
                        #[cfg(target_arch = "wasm32")]
                        file_dialog::web::WebFileDialog::save_plot_ui(&app.plot);

                        ui.close_kind(UiKind::Menu);
                    }

                    // Option to export just the raw plot data
                    if ui.button("Plot Data").clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        file_dialog::native::NativeFileDialog::save_plot_data(
                            app.plot.stored_plot_files(),
                            #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
                            app.mqtt.mqtt_plot_data.as_ref(),
                        );
                        #[cfg(target_arch = "wasm32")]
                        file_dialog::web::WebFileDialog::save_plot_data(
                            app.plot.stored_plot_files(),
                        );
                        ui.close_kind(UiKind::Menu);
                    }
                },
            );

            if ui
                .button(RichText::new(format!(
                    "{icon} Download",
                    icon = egui_phosphor::regular::DOWNLOAD_SIMPLE
                )))
                .clicked()
            {
                app.show_download_window = true;
            }

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

            show_theme_toggle_buttons(ui);
            ui.add(Hyperlink::from_label_and_url(
                "Homepage",
                "https://github.com/luftkode/plotinator3000",
            ));

            #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
            crate::profiling::ui_add_keep_repainting_checkbox(ui, &mut app.keep_repainting);

            if cfg!(target_arch = "wasm32") {
                ui.label(format!("Plotinator3000 v{}", env!("CARGO_PKG_VERSION")));
            }

            #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
            show_mqtt_connect_button(app, ctx, ui);
            collapsible_instructions(ui);
        });
    });
}

#[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
fn show_mqtt_connect_button(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let mqtt_connect_button_txt = if app.mqtt.active_and_connected() {
        RichText::new(format!(
            "{} MQTT connect",
            egui_phosphor::regular::WIFI_HIGH
        ))
        .color(Color32::GREEN)
    } else if app.mqtt.active_but_disconnected() {
        RichText::new(format!(
            "{} MQTT connect",
            egui_phosphor::regular::WIFI_SLASH
        ))
        .color(Color32::RED)
    } else {
        RichText::new("MQTT connect".to_owned())
    };
    if app.mqtt.active_but_disconnected() {
        ui.spinner();
    }
    if ui.button(mqtt_connect_button_txt).clicked() {
        app.mqtt.connect();
    }

    if app.mqtt.listener_active() {
        app.mqtt.poll_data();
        ctx.request_repaint_after(Duration::from_millis(50));
    }
    // Show MQTT configuration window if needed
    app.mqtt.show_connect_window(ui);
}

fn configure_text_styles(ctx: &egui::Context, font_size: f32) {
    let mut style = (*ctx.style()).clone();
    for font_id in style.text_styles.values_mut() {
        font_id.size = font_size;
    }
    ctx.set_style(style);
}

/// Displays a toasts notification if logs are added with the names of all added logs
fn notify_if_logs_added(toasts: &mut Toasts, logs: &[SupportedFormat]) {
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
                    "Unparsed bytes for {remainder}:{log_name}",
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

fn show_theme_toggle_buttons(ui: &mut egui::Ui) {
    let mut theme_preference = ui.ctx().options(|opt| opt.theme_preference);

    ui.horizontal(|ui| {
        ui.selectable_value(&mut theme_preference, ThemePreference::Light, "☀");
        ui.selectable_value(&mut theme_preference, ThemePreference::Dark, "🌙 ");
        ui.selectable_value(&mut theme_preference, ThemePreference::System, "💻");
    });

    ui.ctx().set_theme(theme_preference);
}

pub(crate) enum DownloadMessage {
    Success(String), // filename
    Error(String),   // error message
    Progress {
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    Finished,
}

fn show_download_window(app: &mut App, ctx: &egui::Context) {
    if !app.show_download_window {
        return;
    }
    if app.download_manager.in_progress() {
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    egui::Window::new("Download Logs")
        .collapsible(false)
        .resizable(true)
        .open(&mut app.show_download_window)
        .show(ctx, |ui| {
            ui.add_enabled_ui(!app.download_manager.in_progress(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Host:");
                    ui.text_edit_singleline(&mut app.download_host);
                    ui.label("Port:");
                    ui.add_sized(
                        [80.0, 24.0],
                        egui::TextEdit::singleline(&mut app.download_port),
                    );
                });
            });

            ui.separator();

            if app.download_manager.in_progress() {
                ui.vertical_centered(|ui| {
                    ui.add(egui::ProgressBar::new(app.download_manager.progress).show_percentage());
                    ui.label(&app.download_manager.status_text);
                });
            } else if ui.button("Download Latest data").clicked() {
                app.download_manager.start_download(
                    app.download_host.clone(),
                    app.download_port.clone(),
                    ENDPOINT_DOWNLOAD_LATEST.to_owned(),
                );
            } else if ui.button("Download Today's Data").clicked() {
                app.download_manager.start_download(
                    app.download_host.clone(),
                    app.download_port.clone(),
                    ENDPOINT_DOWNLOAD_TODAY.to_owned(),
                );
            }
        });
}

pub struct DownloadManager {
    tx: Sender<DownloadMessage>,
    rx: Receiver<DownloadMessage>,
    in_progress: bool,
    progress: f32,
    status_text: String,
}

impl DownloadManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            in_progress: false,
            progress: 0.0,
            status_text: String::new(),
        }
    }

    pub fn start_download(&mut self, host: String, port: String, endpoint: String) {
        if self.in_progress {
            return;
        }
        self.in_progress = true;
        self.progress = 0.0;
        self.status_text = "Connecting...".to_string();

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = download::download_zip(&host, &port, tx.clone(), &endpoint);
            match result {
                Ok(filename) => {
                    let _ = tx.send(DownloadMessage::Success(filename));
                }
                Err(e) => {
                    let _ = tx.send(DownloadMessage::Error(e.to_string()));
                }
            }
            let _ = tx.send(DownloadMessage::Finished);
        });
    }

    pub(crate) fn poll(&mut self) -> Vec<DownloadMessage> {
        let mut messages = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            if matches!(msg, DownloadMessage::Finished) {
                self.in_progress = false;
            }
            messages.push(msg);
        }
        messages
    }

    pub fn in_progress(&self) -> bool {
        self.in_progress
    }

    pub fn update_progress(&mut self, downloaded: u64, total: u64) {
        if total > 0 {
            self.progress = downloaded as f32 / total as f32;
        }
        self.status_text = format!(
            "{} / {}",
            format_data_size(downloaded as usize),
            format_data_size(total as usize)
        );
    }
}
