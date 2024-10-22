use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use egui::RichText;

use crate::{updater::remove_disable_update_file, APP_NAME};

/// Display a simple window that allows users to re-enable automatic updates
/// or click `continue...` to open the app
///
/// # Returns
/// `true` if updates are re-enabled
/// `false` if they are not (user clicked `continue`)
pub(crate) fn show_simple_updates_are_disabled_window() -> eframe::Result<bool> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        //centered: true,
        ..Default::default()
    };

    let re_enable_updates_local = Arc::new(AtomicBool::new(false));
    let re_enable_updates = re_enable_updates_local.clone();

    eframe::run_simple_native(APP_NAME, options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new("⚠").size(30.0));
                ui.add_space(10.0);

                if re_enable_updates.load(Ordering::SeqCst) {
                    ui.label(
                        RichText::new("Restart to run the updater")
                            .size(18.)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    if ui
                        .button(RichText::new("Close").strong().size(18.0))
                        .clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                } else {
                    ui.label(
                        RichText::new(format!("Updates are currently disabled"))
                            .strong()
                            .size(18.),
                    );
                    ui.add_space(10.0);
                    if ui
                        .button(RichText::new("Re-enable updates").strong().size(18.0))
                        .clicked()
                    {
                        // Remove the disable updates file
                        remove_disable_update_file().expect("Failed to disable updates");
                        re_enable_updates.store(true, Ordering::SeqCst);
                    }
                }

                // Show a "Continue" button to open the GUI
                ui.add_space(10.0);
                if ui
                    .button(RichText::new("Continue...").strong().size(18.0))
                    .clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Enter))
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    })?;

    Ok(re_enable_updates_local.load(Ordering::SeqCst))
}
