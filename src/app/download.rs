use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use crate::{
    App,
    app::download::{
        self,
        connection::{ENDPOINT_DOWNLOAD_LATEST, ENDPOINT_DOWNLOAD_TODAY},
    },
    util::format_data_size,
};

mod connection;

pub(super) fn show_download_window(app: &mut App, ctx: &egui::Context) {
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

pub(crate) enum DownloadMessage {
    Success {
        filename: String,
    },
    Error(String),
    Progress {
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    Finished,
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
        thread::Builder::new()
            .name("downloader".into())
            .spawn(move || {
                let result = connection::download_zip(&host, &port, tx.clone(), &endpoint);
                match result {
                    Ok(filename) => {
                        let _ = tx.send(DownloadMessage::Success { filename });
                    }
                    Err(e) => {
                        let _ = tx.send(DownloadMessage::Error(e.to_string()));
                    }
                }
                let _ = tx.send(DownloadMessage::Finished);
            })
            .expect("Failed spawning download thread");
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
