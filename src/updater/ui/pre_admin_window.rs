use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use egui::{Color32, RichText};

use crate::APP_NAME;

/// On windows, display an informative error that the app needs to be elevated to admin for the update process
///
/// # Returns
/// `true` if user clicked to update now
/// `false` if user did not click to update
pub(crate) fn pre_admin_window_user_clicked_update() -> eframe::Result<bool> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_icon(
            eframe::icon_data::from_png_bytes(crate::APP_ICON).expect("Failed to load icon"),
        ),
        centered: true,

        ..Default::default()
    };

    let continue_clicked = Arc::new(AtomicBool::new(false));

    eframe::run_simple_native(APP_NAME, options, {
        let continue_clicked = Arc::clone(&continue_clicked);
        move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| {
                {
                    ui.vertical_centered(|ui| {
                        ui.heading(
                            RichText::new("Update available!")
                                .size(30.0)
                                .color(Color32::GREEN),
                        );
                        ui.add_space(10.0);

                        // Show a "Update now" button to open the GUI
                        ui.add_space(10.0);
                        if ui
                            .button(RichText::new("Update now").strong().size(18.0))
                            .clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            continue_clicked.store(true, Ordering::SeqCst);
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.label(
                            RichText::new("Note: runs the updater as administrator".to_owned())
                                .size(15.),
                        );
                    });
                }
            });
        }
    })?;

    Ok(continue_clicked.load(Ordering::SeqCst))
}
