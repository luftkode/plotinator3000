use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

use crate::{
    mqtt::{MqttConfigWindow, MqttData, MqttPoint},
    plot::LogPlotUi,
    util::format_data_size,
};
use dropped_files::handle_dropped_files;
use egui::{Color32, Hyperlink, RichText, ScrollArea, TextStyle, ThemePreference};
use egui_notify::Toasts;
use egui_phosphor::regular;
use log_if::prelude::Plotable;

use file_dialog as fd;
use supported_formats::{LoadedFiles, SupportedFormat};

mod dropped_files;
mod file_dialog;

pub mod supported_formats;
mod util;

/// if a log is loaded from content that exceeds this many unparsed bytes:
/// - Show a toasts warning notification
/// - Show warnings in the UI when viewing parse info for the loaded log
pub const WARN_ON_UNPARSED_BYTES_THRESHOLD: usize = 128;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    #[serde(skip)]
    toasts: Toasts,
    #[serde(skip)]
    mqtt_plots: Vec<MqttData>,
    #[serde(skip)]
    mqtt_channel: Option<std::sync::mpsc::Receiver<MqttPoint>>,
    #[serde(skip)]
    mqtt_stop_flag: Arc<AtomicBool>,
    #[serde(skip)]
    broker_validation_receiver: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    #[serde(skip)]
    discovery_handle: Option<std::thread::JoinHandle<()>>,

    // auto scale plot bounds
    auto_scale: bool,

    loaded_files: LoadedFiles,
    plot: LogPlotUi,
    font_size: f32,
    font_size_init: bool,
    error_message: Option<String>,

    #[serde(skip)]
    mqtt_config_window: Option<MqttConfigWindow>,

    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    web_file_dialog: fd::web::WebFileDialog,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    native_file_dialog: fd::native::NativeFileDialog,

    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    keep_repainting: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            toasts: Toasts::default(),
            mqtt_plots: Vec::new(),
            mqtt_channel: None,
            loaded_files: LoadedFiles::default(),
            plot: LogPlotUi::default(),
            font_size: Self::DEFAULT_FONT_SIZE,
            font_size_init: false,
            error_message: None,
            mqtt_config_window: None,
            mqtt_stop_flag: Arc::new(AtomicBool::new(false)),
            broker_validation_receiver: None,
            discovery_handle: None,
            auto_scale: false,

            #[cfg(target_arch = "wasm32")]
            web_file_dialog: fd::web::WebFileDialog::default(),

            #[cfg(not(target_arch = "wasm32"))]
            native_file_dialog: fd::native::NativeFileDialog::default(),

            #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
            keep_repainting: true,
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
        #[cfg(target_arch = "wasm32")]
        if let Err(e) = self
            .web_file_dialog
            .poll_received_files(&mut self.loaded_files)
        {
            self.error_message = Some(e.to_string());
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(e) = self
            .native_file_dialog
            .parse_picked_files(&mut self.loaded_files)
        {
            self.error_message = Some(e.to_string());
        }

        if !self.font_size_init {
            configure_text_styles(ctx, self.font_size);
        }

        show_top_panel(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            notify_if_logs_added(&mut self.toasts, self.loaded_files.loaded());
            self.plot.ui(
                ui,
                &self.loaded_files.take_loaded_files(),
                &mut self.toasts,
                &self.mqtt_plots,
                &mut self.auto_scale,
            );
            if self.plot.plot_count() == 0 {
                // Display the message when plots are shown
                util::draw_empty_state(ui);
            }

            if let Err(e) = handle_dropped_files(ctx, &mut self.loaded_files) {
                self.error_message = Some(e.to_string());
            }

            self.show_error(ui);
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
        self.toasts.show(ctx);
    }
}

impl App {
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
        egui::menu::bar(ui, |ui| {
            if ui
                .button(RichText::new(format!(
                    "{} Reset",
                    egui_phosphor::regular::TRASH
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
                app.mqtt_stop_flag
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                app.mqtt_channel = None;
                app.mqtt_plots.clear();
            }
            if ui
                .button(RichText::new(format!(
                    "{} Open File",
                    egui_phosphor::regular::FOLDER_OPEN
                )))
                .clicked()
            {
                #[cfg(target_arch = "wasm32")]
                app.web_file_dialog.open(ctx.clone());
                #[cfg(not(target_arch = "wasm32"))]
                app.native_file_dialog.open();
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
            collapsible_instructions(ui);
            if ui.button("MQTT").clicked() {
                app.mqtt_config_window = Some(MqttConfigWindow::default());
            }

            if app.mqtt_channel.is_some() {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
            if let Some(rx) = app.mqtt_channel.as_ref() {
                while let Ok(mqtt_point) = rx.try_recv() {
                    log::info!("Got point=[{},{}]", mqtt_point.point.x, mqtt_point.point.y);
                    if let Some(mp) = app
                        .mqtt_plots
                        .iter_mut()
                        .find(|mp| mp.topic == mqtt_point.topic)
                    {
                        mp.data.push(mqtt_point.point);
                    } else {
                        app.mqtt_plots.push(MqttData {
                            topic: mqtt_point.topic,
                            data: vec![mqtt_point.point],
                        });
                    }
                }
            }
            // Show MQTT configuration window if needed
            if app.mqtt_channel.is_none() {
                if let Some(config) = &mut app.mqtt_config_window {
                    let mut connect_clicked = false;
                    egui::Window::new("MQTT Configuration")
                        .open(&mut config.open)
                        .show(ctx, |ui| {
                            ui.group(|ui| {
                                ui.label("MQTT Broker Address");
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(&mut config.broker_ip)
                                        .on_hover_text("IP address, hostname, or mDNS (.local)");
                                    ui.label(":");
                                    ui.text_edit_singleline(&mut config.broker_port)
                                        .on_hover_text("1883 is the default MQTT broker port");
                                });
                                if let Some(status) = &config.broker_status {
                                    match status {
                                        Ok(()) => {
                                            ui.colored_label(
                                                egui::Color32::GREEN,
                                                RichText::new(format!(
                                                    "{} Broker reachable",
                                                    egui_phosphor::regular::CHECK
                                                )),
                                            );
                                        }
                                        Err(err) => {
                                            ui.colored_label(
                                                egui::Color32::RED,
                                                RichText::new(format!(
                                                    "{} {err}",
                                                    egui_phosphor::regular::WARNING_OCTAGON
                                                )),
                                            );
                                        }
                                    }
                                } else if config.validation_in_progress {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.label("Checking broker...");
                                    });
                                }

                                let current_broker_input =
                                    format!("{}:{}", config.broker_ip, config.broker_port);

                                // Detect input changes
                                if current_broker_input != config.previous_broker_input {
                                    config.previous_broker_input = current_broker_input.clone();
                                    config.last_input_change = Some(Instant::now());
                                    config.broker_status = None;
                                }

                                // Debounce and validate after 500ms
                                if let Some(last_change) = config.last_input_change {
                                    if last_change.elapsed() >= Duration::from_millis(500)
                                        && !config.validation_in_progress
                                    {
                                        let (tx, rx) = std::sync::mpsc::channel();
                                        app.broker_validation_receiver = Some(rx);
                                        config.validation_in_progress = true;
                                        config.last_input_change = None;

                                        // Spawn validation thread
                                        let (host, port) =
                                            (config.broker_ip.clone(), config.broker_port.clone());
                                        std::thread::spawn(move || {
                                            let result = crate::mqtt::validate_broker(&host, &port);
                                            tx.send(result).ok();
                                        });
                                    }
                                }

                                // Check for validation results
                                if let Some(receiver) = &mut app.broker_validation_receiver {
                                    if let Ok(result) = receiver.try_recv() {
                                        config.broker_status = Some(result);
                                        config.validation_in_progress = false;
                                        app.broker_validation_receiver = None;
                                    }
                                }
                                ui.label("Topics:");
                                ui.horizontal(|ui| {
                                    ui.text_edit_singleline(&mut config.new_topic);
                                    if ui.button("Add").clicked() && !config.new_topic.is_empty() {
                                        config.topics.push(config.new_topic.clone());
                                        config.new_topic.clear();
                                    }
                                });

                                let discover_enabled = matches!(config.broker_status, Some(Ok(())))
                                    && !config.discovery_active;

                                if let Ok(port_u16) = config.broker_port.parse::<u16>() {
                                    if !config.discovery_active
                                        && ui
                                            .add_enabled(
                                                discover_enabled,
                                                egui::Button::new(format!(
                                                    "{} Discover Topics",
                                                    egui_phosphor::regular::CELL_TOWER
                                                )),
                                            )
                                            .on_hover_text(
                                                "Continuously find topics (subscribes to #)",
                                            )
                                            .clicked()
                                    {
                                        config.discovery_active = true;
                                        config.discovered_topics.clear();
                                        config
                                            .discovery_stop
                                            .store(false, std::sync::atomic::Ordering::SeqCst);

                                        let host = config.broker_ip.clone();
                                        let (tx, rx) = mpsc::channel();

                                        config.discovery_rx = Some(rx);
                                        app.discovery_handle = Some(crate::mqtt::start_discovery(
                                            host,
                                            port_u16,
                                            Arc::clone(&config.discovery_stop),
                                            tx,
                                        ));
                                    }
                                }

                                if config.discovery_active
                                    && ui
                                        .button(format!(
                                            "{} Stop Discovery",
                                            egui_phosphor::regular::CELL_TOWER
                                        ))
                                        .clicked()
                                {
                                    config.discovery_stop.store(true, Ordering::SeqCst);
                                    config.discovery_active = false;
                                }
                                // Show discovery status
                                if config.discovery_active {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.colored_label(Color32::BLUE, "Discovering topics...");
                                    });
                                }

                                // Process incoming topics
                                if let Some(rx) = &mut config.discovery_rx {
                                    while let Ok(topic) = rx.try_recv() {
                                        if topic.starts_with("!ERROR: ") {
                                            config.discovery_active = false;
                                            ui.colored_label(Color32::RED, &topic[8..]);
                                        } else {
                                            config.discovered_topics.insert(topic);
                                        }
                                    }
                                }

                                // Display discovered topics
                                if !config.discovered_topics.is_empty() {
                                    ui.separator();
                                    ui.label(format!(
                                        "Discovered Topics ({})",
                                        config.discovered_topics.len()
                                    ));

                                    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                        let mut topics: Vec<_> =
                                            config.discovered_topics.iter().collect();
                                        topics.sort();

                                        for topic in topics {
                                            ui.horizontal(|ui| {
                                                if ui.selectable_label(false, topic).clicked() {
                                                    if !config.topics.contains(topic) {
                                                        config.topics.push(topic.to_string());
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                            if !config.topics.is_empty() {
                                ui.label("Subscribed Topics:");
                            }
                            for topic in &mut config.topics {
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(RichText::new(egui_phosphor::regular::TRASH))
                                        .clicked()
                                    {
                                        topic.clear();
                                    } else {
                                        ui.label(topic.clone());
                                    }
                                });
                            }
                            config.topics.retain(|s| !s.is_empty());

                            if ui.button("Connect").clicked() {
                                app.auto_scale = true;
                                log::info!("Auto scaling enabled");
                                connect_clicked = true;
                                app.mqtt_stop_flag
                                    .store(false, std::sync::atomic::Ordering::SeqCst);

                                let broker = config.broker_ip.clone();
                                let topics = config.topics.clone();
                                let (tx, rx) = std::sync::mpsc::channel();
                                app.mqtt_channel = Some(rx);
                                let thread_stop_flag = Arc::clone(&app.mqtt_stop_flag);
                                std::thread::spawn(move || {
                                    crate::mqtt::mqtt_receiver(
                                        tx,
                                        broker,
                                        topics,
                                        thread_stop_flag,
                                    );
                                });
                            }
                        });
                    // 4. Cleanup when window closes
                    if (!config.open || connect_clicked) && config.discovery_active {
                        config.discovery_stop.store(true, Ordering::SeqCst);
                        config.discovery_active = false;
                    }
                }
            }
        });
    });
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
