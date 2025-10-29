use eframe::egui;
use egui::{Color32, Id, RichText, Ui};
use std::collections::HashMap;
use std::path::PathBuf;

/// Enum for messages sent from parser threads to the UI.
/// This is the API contract you requested.
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

/// The final status of a parse, stored in the UI.
#[derive(Clone, Debug)]
enum FinalStatus {
    Succeeded(String), // format_name
    Failed(String),    // error_msg
}

/// Struct to hold the UI state for a single file.
#[derive(Clone, Debug)]
struct FileParseStatus {
    file_name: String,
    full_path: PathBuf,
    current_activity: String,
    progress: f32, // 0.0 - 1.0
    final_status: Option<FinalStatus>,
}

/// Encapsulates the state and UI for the parsing window.
pub struct ParseStatusWindow {
    parse_statuses: HashMap<PathBuf, FileParseStatus>,
    show_parsing_window: bool,
}

impl ParseStatusWindow {
    /// Creates a new, empty parsing window state.
    pub fn new() -> Self {
        Self {
            parse_statuses: HashMap::new(),
            show_parsing_window: false,
        }
    }

    /// Public method to tell the window to open.
    pub fn show(&mut self) {
        self.show_parsing_window = true;
    }

    /// Public method to clear all parsing statuses.
    pub fn clear_statuses(&mut self) {
        self.parse_statuses.clear();
    }

    /// Handles a single update message from a parser thread.
    pub fn handle_update(&mut self, update: ParseUpdate) {
        // Any update should show the window
        self.show_parsing_window = true;

        match update {
            ParseUpdate::Started { path } => {
                let file_name = path.file_name().map_or_else(
                    || "Invalid Path".to_string(),
                    |s| s.to_string_lossy().to_string(),
                );
                self.parse_statuses.insert(
                    path.clone(),
                    FileParseStatus {
                        file_name,
                        full_path: path,
                        current_activity: "Queued...".to_string(),
                        progress: 0.0,
                        final_status: None,
                    },
                );
            }
            ParseUpdate::Attempting { path, format_name } => {
                if let Some(status) = self.parse_statuses.get_mut(&path) {
                    status.current_activity = format!("Trying: {format_name}...");
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
                    if let Some(format_name) = status.current_activity.strip_prefix("Parsing: ") {
                        if let Some(bracket_pos) = format_name.rfind(" (") {
                            let name_only = &format_name[..bracket_pos];
                            status.current_activity =
                                format!("Parsing: {} ({}%)", name_only, (progress * 100.0) as u8);
                        }
                    }
                }
            }
            ParseUpdate::Completed { path, final_format } => {
                if let Some(status) = self.parse_statuses.get_mut(&path) {
                    status.final_status = Some(FinalStatus::Succeeded(final_format));
                    status.current_activity = "Completed".to_string();
                    status.progress = 1.0;
                }
            }
            ParseUpdate::Failed { path, error_msg } => {
                if let Some(status) = self.parse_statuses.get_mut(&path) {
                    status.final_status = Some(FinalStatus::Failed(error_msg));
                    status.current_activity = "Failed".to_string();
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
            .default_size([700.0, 400.0])
            .resizable(true)
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

                // --- Header with controls ---
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Active: {} | Completed: {}",
                        active_count, completed_count
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Clear Completed").clicked() {
                            clear_completed = true;
                        }
                    });
                });

                ui.separator();

                // --- Content Area ---
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !active.is_empty() {
                            ui.heading("Active Tasks");
                            self.render_status_grid(ui, &active, window_id.with("active_grid"));
                        }

                        if !completed.is_empty() {
                            ui.add_space(10.0);
                            ui.heading("Completed Tasks");
                            self.render_status_grid(
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

        self.show_parsing_window = is_open;
        if !is_open {
            // If window is closed, clear all statuses
            self.parse_statuses.clear();
        }
    }

    /// Helper function to render a grid of statuses.
    fn render_status_grid(
        &self,
        ui: &mut Ui,
        statuses: &Vec<(&PathBuf, &FileParseStatus)>,
        grid_id: Id,
    ) {
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
                            let icon = egui_phosphor::regular::CHECK_CIRCLE;
                            ui.label(RichText::new(icon).color(Color32::GREEN).size(16.0));
                        }
                        Some(FinalStatus::Failed(_)) => {
                            let icon = egui_phosphor::regular::WARNING_CIRCLE;
                            ui.label(RichText::new(icon).color(Color32::RED).size(16.0));
                        }
                    };

                    // --- Column 2: File Name ---
                    ui.label(&status.file_name)
                        .on_hover_text(status.full_path.to_string_lossy());

                    // --- Column 3: Status ---
                    if let Some(final_status) = &status.final_status {
                        let (msg, color) = match final_status {
                            FinalStatus::Succeeded(fmt) => {
                                (format!("Success: {fmt}"), Color32::GREEN)
                            }
                            FinalStatus::Failed(_) => ("Error".to_string(), Color32::RED),
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
                                egui::ProgressBar::new(status.progress)
                                    .show_percentage()
                                    .desired_width(200.0)
                                    .text(format!("{}%", (status.progress * 100.0) as u32)),
                            );
                        });
                    }
                    ui.end_row();
                }
            });
    }
}
