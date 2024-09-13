use egui::{DroppedFile, Hyperlink, RichText, Stroke};

pub fn file_info(file: &DroppedFile) -> String {
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

pub fn draw_empty_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(100.0);
        ui.heading(RichText::new("Drag and drop logfiles onto this window").size(40.0));
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
                            ui.add(Hyperlink::from_label_and_url(
                                "https://github.com/luftkode/mbed-motor-control",
                                "https://github.com/luftkode/mbed-motor-control",
                            ));

                            ui.end_row();

                            ui.label("• PID Logs");
                            ui.label("Contains PID controller data");
                            ui.end_row();

                            ui.label("• Status Logs");
                            ui.label(
                                "General status information such as engine temperature, and controller state machine information",
                            );
                            ui.end_row();

                            ui.label(RichText::new("Generator").strong());
                            ui.label("Logs from the generator");

                            ui.add(Hyperlink::from_label_and_url(
                                "https://github.com/luftkode/delphi_generator_linux",
                                "https://github.com/luftkode/delphi_generator_linux",
                            ));
                            ui.end_row();
                        });
            });
    });
}
