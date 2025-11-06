use eframe::egui;
use egui::{Color32, Id, Key, ProgressBar, RichText, Ui};
use egui_phosphor::regular::{CHECK_CIRCLE, WARNING_CIRCLE};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

/// Enum for messages sent from parser threads to the UI.
#[derive(Debug, Clone)]
pub enum ParseUpdate {
    /// Sent when a file is first discovered.
    Started { path: PathBuf },
    /// Sent when a specific format parser is being tried.
    Attempting { path: PathBuf, format_name: String },
    /// Sent when a format is confirmed and deep parsing begins.
    Confirmed { path: PathBuf, format_name: String },
    /// Sent periodically during a long parse.
    Progress { path: PathBuf, progress: f32 }, // 0.0 to 1.0
    /// Sent on successful completion.
    Completed { path: PathBuf, final_format: String },
    /// Sent on failure.
    Failed { path: PathBuf, error_msg: String },
}

#[derive(Debug, Clone)]
pub struct UpdateChannel {
    tx: Sender<ParseUpdate>,
}

impl UpdateChannel {
    pub fn new(tx: Sender<ParseUpdate>) -> Self {
        Self { tx }
    }

    pub fn send(&self, update: ParseUpdate) {
        self.tx.send(update).expect("receiver dropped channel");
    }
}

/// The final status of a parse, stored in the UI.
#[derive(Clone, Debug)]
enum FinalStatus {
    Succeeded(String), // name of the format e.g. `Njord Altimeter`
    Failed(String),    // error message
}

/// Struct to hold the UI state for a single file.
#[derive(Clone, Debug)]
struct FileParseStatus {
    file_name: String,
    full_path: PathBuf,
    current_activity: String,
    progress: f32, // 0.0 - 1.0
    final_status: Option<FinalStatus>,
    time_started: Instant,
    total_parse_time: Option<Duration>,
}

impl FileParseStatus {
    fn time_elapsed(&self) -> Duration {
        self.time_started.elapsed()
    }

    fn time_elapsed_seconds(&self) -> String {
        format!("{:.1?}s", self.time_elapsed().as_secs_f32())
    }

    fn total_parse_time_seconds(&self) -> String {
        debug_assert_ne!(self.total_parse_time, None);
        if let Some(total_time) = self.total_parse_time.as_ref() {
            format!("{total_time:.1?}")
        } else {
            log::error!("Got parse time when completed parsing time was not assigned");
            String::new()
        }
    }
}

/// Encapsulates the state and UI for the parsing window.
#[derive(Debug, Default)]
pub struct ParseStatusWindow {
    parse_statuses: HashMap<PathBuf, FileParseStatus>,
    show_parsing_window: bool,
}

impl ParseStatusWindow {
    /// Open the window.
    pub fn show(&mut self) {
        self.show_parsing_window = true;
    }

    pub fn is_open(&self) -> bool {
        self.show_parsing_window
    }

    /// Clear all parsing statuses.
    pub fn clear_statuses(&mut self) {
        self.parse_statuses.clear();
    }

    /// Handles a single update message from a parser thread.
    pub fn handle_update(&mut self, update: ParseUpdate) {
        log::info!("Received Parser update: {update:?}");
        // Any update should show the window
        self.show_parsing_window = true;

        match update {
            ParseUpdate::Started { path } => {
                let file_name = path.file_name().map_or_else(
                    || "Invalid Path".to_owned(),
                    |s| s.to_string_lossy().to_string(),
                );
                self.parse_statuses.insert(
                    path.clone(),
                    FileParseStatus {
                        file_name,
                        full_path: path,
                        current_activity: "Queued...".to_owned(),
                        progress: 0.0,
                        final_status: None,
                        time_started: Instant::now(),
                        total_parse_time: None,
                    },
                );
            }
            ParseUpdate::Attempting { path, format_name } => {
                if let Some(status) = self.parse_statuses.get_mut(&path) {
                    status.current_activity = format!("Parsing: {format_name}...");
                    status.progress = 0.0;
                }
            }
            ParseUpdate::Confirmed { path, format_name } => {
                if let Some(status) = self.parse_statuses.get_mut(&path) {
                    status.current_activity = format!("Parsing: {format_name} (0%)");
                    status.progress = 0.0;
                }
            }
            ParseUpdate::Progress { path, progress } => {
                if let Some(status) = self.parse_statuses.get_mut(&path) {
                    status.progress = progress;
                    if let Some(format_name) = status.current_activity.strip_prefix("Parsing: ")
                        && let Some(bracket_pos) = format_name.rfind(" (")
                    {
                        let name_only = &format_name[..bracket_pos];
                        status.current_activity =
                            format!("Parsing: {name_only} ({}%)", (progress * 100.0) as u8);
                    }
                }
            }
            ParseUpdate::Completed { path, final_format } => {
                if let Some(status) = self.parse_statuses.get_mut(&path) {
                    status.total_parse_time = Some(status.time_elapsed());
                    status.final_status = Some(FinalStatus::Succeeded(final_format));
                    status.current_activity = "Completed".to_owned();
                    status.progress = 1.0;
                }
            }
            ParseUpdate::Failed { path, error_msg } => {
                if let Some(status) = self.parse_statuses.get_mut(&path) {
                    status.total_parse_time = Some(status.time_elapsed());
                    status.final_status = Some(FinalStatus::Failed(error_msg));
                    status.current_activity = "Failed".to_owned();
                    status.progress = 0.0;
                }
            }
        }
    }

    /// Renders the pop-up window.
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.show_parsing_window {
            return;
        }

        let mut is_open = self.show_parsing_window;
        let window_id = Id::new("parsing_status_window");
        let mut clear_completed = false;

        egui::Window::new("File Parsing Status")
            .id(window_id)
            .open(&mut is_open)
            .auto_sized()
            .show(ctx, |ui| {
                // --- Separate active and completed tasks ---
                let mut active = Vec::new();
                let mut completed = Vec::new();
                for (path, status) in &self.parse_statuses {
                    if status.final_status.is_none() {
                        active.push((path, status));
                    } else {
                        completed.push((path, status));
                    }
                }
                active.sort_by(|a, b| a.1.file_name.cmp(&b.1.file_name));
                completed.sort_by(|a, b| a.1.file_name.cmp(&b.1.file_name));

                let active_count = active.len();
                let completed_count = completed.len();

                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Active: {active_count} | Completed: {completed_count}"
                    ));
                    ui.add_space(40.);
                    if ui.button("Clear Completed").clicked() {
                        clear_completed = true;
                    }
                });

                // --- Content Area ---
                egui::ScrollArea::vertical()
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        if !active.is_empty() {
                            ui.heading("Active Tasks");
                            Self::render_status_grid(ui, &active, window_id.with("active_grid"));
                        }

                        if !completed.is_empty() {
                            ui.add_space(10.0);
                            ui.heading("Completed Tasks");
                            Self::render_status_grid(
                                ui,
                                &completed,
                                window_id.with("completed_grid"),
                            );
                        }
                    });

                // Defer the mutation until after the iteration
                if clear_completed {
                    self.parse_statuses
                        .retain(|_path, status| status.final_status.is_none());
                }
            });
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            is_open = false;
        }
        self.show_parsing_window = is_open;
    }

    /// Helper function to render a grid of statuses.
    fn render_status_grid(ui: &mut Ui, statuses: &Vec<(&PathBuf, &FileParseStatus)>, grid_id: Id) {
        egui::Grid::new(grid_id)
            .num_columns(3)
            .striped(true)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                for (_path, status) in statuses {
                    // --- Column 1: Icon ---
                    match &status.final_status {
                        None => {
                            ui.add(egui::Spinner::new().size(16.0));
                        }
                        Some(FinalStatus::Succeeded(_)) => {
                            let icon = CHECK_CIRCLE;
                            ui.label(RichText::new(icon).color(Color32::GREEN).size(16.0));
                        }
                        Some(FinalStatus::Failed(_)) => {
                            let icon = WARNING_CIRCLE;
                            ui.label(RichText::new(icon).color(Color32::RED).size(16.0));
                        }
                    };

                    // --- Column 2: File Name ---
                    ui.label(&status.file_name)
                        .on_hover_text(status.full_path.to_string_lossy());

                    // --- Column 3: Status ---
                    if let Some(final_status) = &status.final_status {
                        let (msg, color) = match final_status {
                            FinalStatus::Succeeded(fmt) => (
                                format!("{fmt} in {}", status.total_parse_time_seconds()),
                                Color32::GREEN,
                            ),
                            FinalStatus::Failed(_) => ("Error".to_owned(), Color32::RED),
                        };
                        let label = ui.label(RichText::new(msg).color(color));
                        if let FinalStatus::Failed(err) = final_status {
                            label.on_hover_text(err);
                        }
                    } else {
                        // Active: show progress bar + text
                        ui.vertical(|ui| {
                            ui.label(&status.current_activity)
                                .on_hover_text(&status.current_activity);
                            ui.add(
                                ProgressBar::new(status.progress)
                                    .show_percentage()
                                    .desired_width(200.0)
                                    .text(format!(
                                        "{}% - {}",
                                        (status.progress * 100.0) as u32,
                                        status.time_elapsed_seconds()
                                    )),
                            );
                        });
                    }
                    ui.end_row();
                }
            });
    }
}
