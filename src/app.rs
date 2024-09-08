use crate::{
    logs::{
        mbed_motor_control::{
            pid::{PidLog, PidLogHeader},
            status::{StatusLog, StatusLogHeader},
            MbedMotorControlLogHeader,
        },
        Log,
    },
    plot::LogPlot,
};
use egui::{DroppedFile, Hyperlink, RichText, Stroke};
use std::{fs, io::BufReader};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    dropped_files: Vec<DroppedFile>,
    picked_path: Option<String>,
    pid_log: Option<PidLog>,
    status_log: Option<StatusLog>,
    plot: LogPlot,
    font_size: f32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            dropped_files: Vec::new(),
            picked_path: None,
            pid_log: None,
            status_log: None,
            plot: LogPlot::default(),
            font_size: 16.0,
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

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
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }
                if ui.button("Reset").clicked() {
                    *self = Self::default();
                }
                ui.label("Font size:");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.font_size)
                            .speed(0.1)
                            .range(8.0..=32.0)
                            .suffix("px"),
                    )
                    .changed()
                {
                    // Update the font size for all text styles
                    let mut style = (*ctx.style()).clone();
                    for (_text_style, font_id) in style.text_styles.iter_mut() {
                        font_id.size = self.font_size;
                    }
                    ctx.set_style(style);
                }
                egui::widgets::global_dark_light_mode_buttons(ui);
                ui.add(Hyperlink::from_label_and_url(
                    "Homepage",
                    "https://github.com/luftkode/logviewer-rs",
                ));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            self.plot
                .ui(ui, self.pid_log.as_ref(), self.status_log.as_ref());

            ui.separator();

            if self.dropped_files.is_empty() {
                // Display the message when no files have been dropped and no logs are loaded
                self.draw_empty_state(ui);
            } else {
                ui.group(|ui| {
                    ui.label("Dropped files:");
                    for file in &self.dropped_files {
                        ui.label(file_info(file));
                    }
                });
            }

            preview_files_being_dropped(ctx);
            // Collect dropped files:
            ctx.input(|i| {
                if !i.raw.dropped_files.is_empty() {
                    self.dropped_files.clone_from(&i.raw.dropped_files);
                    self.parse_dropped_files();
                }
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

// Utility functions (not the primary framework functions such as `update` and `save`)
impl App {
    fn parse_dropped_files(&mut self) {
        // The `to_vec` copies but is needed here to prevent a mutable borrow of self in the loop
        #[allow(clippy::unnecessary_to_owned)]
        for file in self.dropped_files.to_vec() {
            self.parse_file(&file);
        }
    }

    fn parse_file(&mut self, file: &DroppedFile) {
        if let Some(content) = file.bytes.as_ref().map(|b| b.as_ref()) {
            // This is how content is made accesible via drag-n-drop in a browser
            self.parse_content(content);
        } else if let Some(path) = &file.path {
            // This is how content is accesible via drag-n-drop when the app is running natively
            self.parse_path(path);
        }
    }

    fn parse_content(&mut self, mut content: &[u8]) {
        if self.pid_log.is_none() && PidLogHeader::is_buf_header(content).unwrap_or(false) {
            self.pid_log = PidLog::from_reader(&mut content).ok();
        } else if self.status_log.is_none()
            && StatusLogHeader::is_buf_header(content).unwrap_or(false)
        {
            self.status_log = StatusLog::from_reader(&mut content).ok();
        }
    }

    fn parse_path(&mut self, path: &std::path::Path) {
        if self.pid_log.is_none() && PidLogHeader::file_starts_with_header(path).unwrap_or(false) {
            self.pid_log = fs::File::open(path)
                .ok()
                .and_then(|file| PidLog::from_reader(&mut BufReader::new(file)).ok());
        } else if self.status_log.is_none()
            && StatusLogHeader::file_starts_with_header(path).unwrap_or(false)
        {
            self.status_log = fs::File::open(path)
                .ok()
                .and_then(|file| StatusLog::from_reader(&mut BufReader::new(file)).ok());
        }
    }

    fn draw_empty_state(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading("Drag and drop logfiles onto this window");
            ui.add_space(40.0);

            let table_width = ui.available_width() * 0.8;
            egui::Frame::none()
                .fill(ui.style().visuals.extreme_bg_color)
                .stroke(Stroke::new(1.0, ui.style().visuals.widgets.active.bg_fill))
                .inner_margin(10.0)
                .outer_margin(0.0)
                .show(ui, |ui| {
                    ui.set_width(table_width);
                    egui::Grid::new("supported_formats_grid")
                        .num_columns(2)
                        .spacing([40.0, 10.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.colored_label(
                                ui.style().visuals.strong_text_color(),
                                "Supported Formats",
                            );
                            ui.colored_label(
                                ui.style().visuals.strong_text_color(),
                                "Description",
                            );
                            ui.end_row();

                            ui.label(RichText::new("Mbed Motor Control").strong());
                            ui.label("Logs from Mbed-based motor controller");
                            ui.end_row();

                            ui.label("• PID Logs");
                            ui.label("Contains PID controller data");
                            ui.end_row();

                            ui.label("• Status Logs");
                            ui.label(
                                "General status information such as engine temperature, and controller state machine information",
                            );
                            ui.end_row();
                        });
                });
        });
    }
}

/// Preview hovering files:
fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}

fn file_info(file: &DroppedFile) -> String {
    let path = file
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .or_else(|| (!file.name.is_empty()).then(|| file.name.clone()))
        .unwrap_or_else(|| "???".to_owned());

    let mut info = vec![path];
    if !file.mime.is_empty() {
        info.push(format!("type: {}", file.mime));
    }
    if let Some(bytes) = &file.bytes {
        info.push(format!("{} bytes", bytes.len()));
    }

    info.join(" ")
}
