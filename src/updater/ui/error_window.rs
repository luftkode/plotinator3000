use egui::{Color32, RichText};

use crate::APP_NAME;

#[allow(dead_code)]
/// Display a simple window with the error that occurred
pub(crate) fn show_error_occurred(err_msg: &str) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([300.0, 300.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(crate::APP_ICON).expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    eframe::run_simple_native(APP_NAME, options, {
        let err_msg = err_msg.to_owned();
        move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let err_msg = err_msg.clone();
                {
                    ui.vertical_centered(|ui| {
                        ui.heading(RichText::new("⚠").size(30.0).color(Color32::RED));
                        ui.add_space(10.0);

                        ui.label(RichText::new("An error occurred: ").size(18.).strong());
                        ui.separator();
                        ui.label(RichText::new(err_msg).strong().size(20.));
                        ui.separator();
                        ui.add_space(10.0);
                        ui.label("Please report this error at the link below");
                        ui.add(egui::Hyperlink::from_label_and_url(
                            "Plotinator3000 issues",
                            "https://github.com/luftkode/plotinator3000/issues",
                        ));

                        ui.add_space(10.0);
                        if ui
                            .button(RichText::new("Close").strong().size(18.0))
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                }
            });
        }
    })
    .expect("Failed launching error window");
}
