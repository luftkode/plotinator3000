use egui::{Color32, RichText};
use egui_phosphor::regular;
use loaded_logs::LoadedLogsUi;

use crate::app::PlayBackButtonEvent;

use super::{
    axis_config::AxisConfig, play_state::PlayState, plot_visibility_config::PlotVisibilityConfig,
};

pub mod loaded_logs;

pub fn show_settings_grid(
    ui: &mut egui::Ui,
    play_state: &PlayState,
    playback_button_event: &mut Option<PlayBackButtonEvent>,
    line_width: &mut f32,
    axis_cfg: &mut AxisConfig,
    plot_visibility_cfg: &mut PlotVisibilityConfig,
    mut loaded_logs_ui: LoadedLogsUi<'_>,
) {
    egui::Grid::new("settings").show(ui, |ui| {
        ui.label("Line width");
        ui.add(
            egui::DragValue::new(line_width)
                .speed(0.02)
                .range(0.5..=20.0),
        );
        ui.horizontal_top(|ui| {
            axis_cfg.toggle_axis_cfg_ui(ui);
            ui.label("|");
            plot_visibility_cfg.toggle_visibility_ui(ui);
        });

        ui.horizontal_centered(|ui| {
            ui.label("| ");
            // Reset button
            let reset_text = RichText::new(egui_phosphor::regular::REWIND);
            if ui.button(reset_text).clicked() {
                *playback_button_event = Some(PlayBackButtonEvent::Reset);
            }
            let playpause_text = if play_state.is_playing() {
                RichText::new(regular::PAUSE).color(Color32::YELLOW)
            } else {
                RichText::new(regular::PLAY).color(Color32::GREEN)
            };
            if ui.button(playpause_text).clicked() {
                *playback_button_event = Some(PlayBackButtonEvent::PlayPause);
            }

            ui.label(RichText::new(play_state.formatted_time()));
            ui.label(" |");
        });

        ui.end_row();
    });
    loaded_logs_ui.show(ui);
}
