use egui::{Frame, Hyperlink, Label, RichText, ScrollArea, Stroke, Ui};
use egui_phosphor::regular;

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

const LOG_FORMATS: &[LogFormat<'_>] = &[
    LogFormat {
        title: "Mbed Motor Control",
        description: "Logs from the Mbed-based Swiss motor controller.",
        link: "https://github.com/luftkode/mbed-motor-control",
        subitems: Some(&[
            SubLogFormat {
                title: "PID Logs",
                description: "Contains PID controller data.",
            },
            SubLogFormat {
                title: "Status Logs",
                description: "General status information such as engine temperature, and controller state machine information.",
            },
        ]),
    },
    LogFormat {
        title: "Generator",
        description: "Logs from the generator.",
        link: "https://github.com/luftkode/delphi_generator_linux",
        subitems: None,
    },
    LogFormat {
        title: "Navsys",
        description: "Navsys.sps logs with data from GPS, Magsensor, Altimeter, etc.",
        link: "https://github.com/luftkode/navsys",
        subitems: Some(&[SubLogFormat {
            title: "Kitchen sinks of Navsys entries",
            description: "Navsys-like files with any kind of data that could be present in Navsys.sps files. For example a file with only Mag data.",
        }]),
    },
];

#[cfg(not(target_arch = "wasm32"))]
const HDF5_LOG_FORMATS: &[LogFormat<'_>] = &[
    LogFormat {
        title: "Bifrost TX Loop Current",
        description: "Loop Current measurements.",
        link: "https://github.com/luftkode/bifrost-app",
        subitems: None,
    },
    LogFormat {
        title: "Njord Altimeter",
        description: "Height measurements from the Njord Altimeter.",
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
        description: "Height measurements.",
        link: "https://github.com/luftkode/frame-altimeter",
        subitems: None,
    },
    LogFormat {
        title: "Frame Inclinometers",
        description: "Frame Pitch and Roll in degrees.",
        link: "https://github.com/luftkode/frame-inclinometer",
        subitems: None,
    },
    LogFormat {
        title: "Frame Magnetometer",
        description: "B-field measurements.",
        link: "https://github.com/luftkode/frame-magnetometer",
        subitems: None,
    },
    LogFormat {
        title: "TSC",
        description: "GPS data from the TS (TEM data is not supported)",
        link: "https://github.com/luftkode/tib3d-script",
        subitems: None,
    },
];

/// Draws a single log format entry
fn draw_log_format_entry(ui: &mut Ui, format: &LogFormat<'_>) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format.title).strong());
            ui.add_space(8.0);
            ui.add(Hyperlink::from_label_and_url(
                RichText::new(format!("{} Repository", regular::GITHUB_LOGO)),
                format.link,
            ));
        });

        let description_text = RichText::new(format.description)
            .italics()
            .color(ui.visuals().text_color().gamma_multiply(0.8));
        ui.add(Label::new(description_text).wrap());

        if let Some(subitems) = format.subitems {
            ui.indent("subitems", |ui| {
                for sub in subitems {
                    ui.label(RichText::new(format!(
                        "• {}: {}",
                        sub.title, sub.description
                    )));
                }
            });
        }
    });
    ui.add_space(6.0);
}

pub fn draw_empty_state(ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.add_space(20.0);
        ui.heading(RichText::new("Drag and drop files, directories, or zip archives").size(30.));
        ui.add_space(16.0);

        // Calculate available height for the scroll area
        let available_height = ui.available_height() - 60.0; // Reserve space for the heading above

        Frame::new()
            .fill(ui.style().visuals.extreme_bg_color)
            .stroke(Stroke::new(1.0, ui.style().visuals.widgets.active.bg_fill))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width() * 0.9);

                ui.heading(RichText::new("Supported Formats").size(30.));
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(8.0);

                ScrollArea::vertical()
                    .max_height(available_height)
                    .show(ui, |ui| {
                        ui.heading("Non-HDF5 Formats");
                        ui.add_space(6.0);

                        for format in LOG_FORMATS {
                            draw_log_format_entry(ui, format);
                            ui.separator();
                            ui.add_space(6.0);
                        }

                        // Section for HDF5 formats
                        ui.heading("HDF5 Formats");
                        ui.add_space(6.0);

                        #[cfg(target_arch = "wasm32")]
                        {
                            ui.label(
                                RichText::new("⚠ HDF5 is not supported on web builds.")
                                    .color(egui::Color32::RED),
                            );
                        }

                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            for format in HDF5_LOG_FORMATS {
                                draw_log_format_entry(ui, format);
                                ui.separator();
                                ui.add_space(6.0);
                            }
                        }
                    });
            });
    });
}
