use std::time::Duration;

use plotinator_download::endpoint::Endpoint;

use crate::App;

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
                    ui.add(
                        egui::ProgressBar::new(app.download_manager.progress()).show_percentage(),
                    );
                    ui.label(app.download_manager.status_text().to_owned());
                });
            } else if ui.button("Download Latest data").clicked() {
                app.download_manager.start_download(
                    app.download_host.clone(),
                    app.download_port.clone(),
                    Endpoint::DownloadLatestData,
                );
            } else if ui.button("Download Today's Data").clicked() {
                app.download_manager.start_download(
                    app.download_host.clone(),
                    app.download_port.clone(),
                    Endpoint::DownloadTodaysData,
                );
            }
        });
}
