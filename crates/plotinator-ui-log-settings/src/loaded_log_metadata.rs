use egui::RichText;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub struct LoadedLogMetadata {
    description: String,
    value: String,
    selected: bool,
}
impl LoadedLogMetadata {
    pub fn new(description: String, value: String) -> Self {
        Self {
            description,
            value,
            selected: false,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new(&self.description).strong());
        if self.value.len() > 100 {
            let shortened_preview_value = format!("{} ...", &self.value[..40]);
            if ui.button(&shortened_preview_value).clicked() {
                self.selected = !self.selected;
            };
            if self.selected {
                egui::Window::new(shortened_preview_value)
                    .open(&mut self.selected)
                    .show(ui.ctx(), |ui| {
                        ui.horizontal_wrapped(|ui| ui.label(&self.value));
                    });
            }
        } else {
            ui.label(&self.value);
        }

        ui.end_row();
    }
}
