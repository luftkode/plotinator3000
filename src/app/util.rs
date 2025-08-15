use egui::{Hyperlink, RichText, Stroke};

struct LogFormat<'s> {
    title: &'s str,
    description: &'s str,
    link: &'s str,
    subitems: Option<&'s [SubLogFormat<'s>]>,
}

struct SubLogFormat<'s> {
    title: &'s str,
    description: &'s str,
}

fn draw_log_formats(ui: &mut egui::Ui, formats: &[LogFormat<'_>]) {
    for format in formats {
        ui.label(RichText::new(format.title).strong());
        ui.label(format.description);

        ui.add(Hyperlink::from_label_and_url(format.link, format.link));

        ui.end_row();
        if let Some(subformats) = format.subitems {
            for sub in subformats {
                ui.label(format!("• {}", sub.title));
                ui.label(sub.description);
                ui.end_row();
            }
        }
    }
}

pub fn draw_empty_state(gui: &mut egui::Ui) {
    gui.vertical_centered(|arg_ui| {
        arg_ui.add_space(100.0);
        arg_ui.heading(
            RichText::new("Drag and drop files, directories/folder, or zip archives onto this window")
                .size(40.0),
        );
        arg_ui.add_space(40.0);

        let table_width = arg_ui.available_width() * 0.8;
        egui::Frame::new()
            .fill(arg_ui.style().visuals.extreme_bg_color)
            .stroke(Stroke::new(1.0, arg_ui.style().visuals.widgets.active.bg_fill))
            .inner_margin(10.0)
            .outer_margin(0.0)
            .show(arg_ui, |inner_arg_ui| {
                inner_arg_ui.set_width(table_width);
                egui::Grid::new("supported_formats_grid")
                    .num_columns(2)
                    .spacing([40.0, 10.0])
                    .striped(true)
                    .show(inner_arg_ui, |ui| {
                        ui.colored_label(
                            ui.style().visuals.strong_text_color(),
                            "Supported Formats",
                        );
                        ui.colored_label(
                            ui.style().visuals.strong_text_color(),
                            "Description",
                        );
                        ui.end_row();

                        let log_formats = [
                            LogFormat {
                                title: "Mbed Motor Control",
                                description: "Logs from Mbed-based motor controller",
                                link: "https://github.com/luftkode/mbed-motor-control",
                                subitems: Some(&[
                                    SubLogFormat {
                                        title: "PID Logs",
                                        description: "Contains PID controller data",
                                    },
                                    SubLogFormat {
                                        title: "Status Logs",
                                        description:
                                            "General status information such as engine temperature, and controller state machine information",
                                    },
                                ]),
                            },
                            LogFormat {
                                title: "Generator",
                                description: "Logs from the generator",
                                link: "https://github.com/luftkode/delphi_generator_linux",
                                subitems: None,
                            },
                            LogFormat {
                                title: "Navsys",
                                description:
                                    "Navsys.sps logs with data from GPS, Magsensor, Altimeter, etc.",
                                link: "https://github.com/luftkode/navsys",
                                subitems: Some(&[
                                    SubLogFormat {
                                        title: "Kitchen sinks of Navsys entries",
                                        description: "Navsys-like files with any kind of data that could be present in Navsys.sps files. For example a file with only Mag data."
                                    }
                                ]),
                            },
                        ];

                        draw_log_formats(ui, &log_formats);
                        list_supported_hdf5_formats(ui);
                    });
            });
    });
}

fn list_supported_hdf5_formats(ui: &mut egui::Ui) {
    #[cfg(target_arch = "wasm32")]
    ui.label(RichText::new("⚠ No HDF5 support on web ⚠"));
    #[cfg(not(target_arch = "wasm32"))]
    {
        let log_formats = [
            LogFormat {
                title: "Bifrost TX Loop Current",
                description: "Loop Current measurements",
                link: "https://github.com/luftkode/bifrost-app",
                subitems: None,
            },
            LogFormat {
                title: "Njord Altimeter",
                description: "Height measurements from the Njord Altimeter",
                link: "https://github.com/luftkode/njord-altimeter",
                subitems: None,
            },
            LogFormat {
                title: "Njord INS",
                description: "Angle, position, speed, etc.",
                link: "https://github.com/luftkode/njord-ins",
                subitems: None,
            },
            LogFormat {
                title: "Frame Altimeters",
                description: "Height measurements",
                link: "https://github.com/luftkode/frame-altimeter",
                subitems: None,
            },
            LogFormat {
                title: "Frame Inclinometers",
                description: "Frame Pitch and Roll in degrees",
                link: "https://github.com/luftkode/frame-inclinometer",
                subitems: None,
            },
            LogFormat {
                title: "Frame Magnetometer",
                description: "B-field measurements",
                link: "https://github.com/luftkode/frame-magnetometer",
                subitems: None,
            },
        ];
        draw_log_formats(ui, &log_formats);
    }
}
