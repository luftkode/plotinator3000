use egui::{Response, RichText};
use egui_phosphor::regular;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub struct MipMapSettings {
    enabled: bool,
    auto_set: bool,
    level: usize,
}

impl Default for MipMapSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_set: true,
            level: 0,
        }
    }
}

impl MipMapSettings {
    /// Render the [`MipMapSettings`] part of the UI
    pub fn show(&mut self, ui: &mut egui::Ui) -> Response {
        egui::Grid::new("mipmap settings").show(ui, |ui| {

            ui.toggle_value(&mut self.enabled, RichText::new(format!("{} Downsampling", regular::EQUALIZER))).on_hover_text("Enable downsampling with min/max mipmaps. Will show 2 plots per logical plot, one that displays the minimum values and one with the maximum values");
            ui.add_enabled_ui(self.enabled, |ui| {
                ui.checkbox(&mut self.auto_set, "auto")
                .on_hover_text("Toggle auto-scaling mipmap'ing (downsampling)");

                ui.add_enabled_ui(!self.auto_set, |ui| {
                    let slider_resp = ui.add(
                        egui::DragValue::new(&mut self.level)
                        .speed(1)
                        .range(0..=32)
                        .suffix(" lvls"),
                    ).on_hover_text("Manually set the downsampling level").on_disabled_hover_text("Manually set the downsampling level (requires disabling auto)");

                    if slider_resp.changed() {
                        log::info!("Mip map level changed to: {}", self.level);
                    }
                });

            ui.label("|");
        })
    })
        .response
    }

    /// Return the current configuration as a [`plot_util::MipMapSetting`].
    pub fn configuration(&self) -> plot_util::MipMapConfiguration {
        if self.enabled {
            plot_util::MipMapConfiguration::Enabled(if self.auto_set {
                None
            } else {
                Some(self.level)
            })
        } else {
            plot_util::MipMapConfiguration::Disabled
        }
    }
}
