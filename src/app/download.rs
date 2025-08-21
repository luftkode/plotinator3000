use std::time::Duration;

use egui_notify::Toasts;
use plotinator_download::{
    DATA_BINDER_PORT, DownloadMessage, TS_IP, endpoint::Endpoint, manager::DownloadManager,
};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(crate) struct DownloadUi {
    pub(crate) show: bool,
    host: String,
    port: String,

    #[serde(skip)]
    download_manager: DownloadManager,
}

impl Default for DownloadUi {
    fn default() -> Self {
        Self {
            show: false,
            host: TS_IP.to_owned(),
            port: DATA_BINDER_PORT.to_owned(),
            download_manager: Default::default(),
        }
    }
}

impl DownloadUi {
    pub(super) fn show_download_window(&mut self, ctx: &egui::Context) {
        if !self.show {
            return;
        }
        if self.download_manager.in_progress() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        egui::Window::new("Download Logs")
            .collapsible(false)
            .resizable(true)
            .open(&mut self.show)
            .show(ctx, |ui| {
                ui.add_enabled_ui(!self.download_manager.in_progress(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Host:");
                        ui.text_edit_singleline(&mut self.host);
                        ui.label("Port:");
                        ui.add_sized([80.0, 24.0], egui::TextEdit::singleline(&mut self.port));
                    });
                });

                ui.separator();

                if self.download_manager.in_progress() {
                    ui.vertical_centered(|ui| {
                        ui.add(
                            egui::ProgressBar::new(self.download_manager.progress())
                                .show_percentage(),
                        );
                        ui.label(self.download_manager.status_text().to_owned());
                    });
                } else if ui.button("Download Latest data").clicked() {
                    self.download_manager.start_download(
                        self.host.clone(),
                        self.port.clone(),
                        Endpoint::DownloadLatestData,
                    );
                } else if ui.button("Download Today's Data").clicked() {
                    self.download_manager.start_download(
                        self.host.clone(),
                        self.port.clone(),
                        Endpoint::DownloadTodaysData,
                    );
                }
            });
    }

    pub(super) fn poll_download_messages(&mut self, ctx: &egui::Context, toasts: &mut Toasts) {
        for msg in self.download_manager.poll() {
            match msg {
                DownloadMessage::Success { filename } => {
                    toasts
                        .success(format!("Downloaded: {filename}"))
                        .duration(Some(Duration::from_secs(5)));
                }
                DownloadMessage::Error(err) => {
                    toasts
                        .error(format!("Download failed: {err}"))
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
    }
}
