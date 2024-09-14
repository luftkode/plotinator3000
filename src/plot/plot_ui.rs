use egui::{Color32, RichText};
use egui_phosphor::regular;

use crate::app::PlayBackButtonEvent;

use super::{
    axis_config::AxisConfig, play_state::PlayState, plot_visibility_config::PlotVisibilityConfig,
};

pub fn show_settings_grid(
    gui: &mut egui::Ui,
    play_state: &PlayState,
    playback_button_event: &mut Option<PlayBackButtonEvent>,
    line_width: &mut f32,
    axis_cfg: &mut AxisConfig,
    plot_visibility_cfg: &mut PlotVisibilityConfig,
) {
    _ = egui::Grid::new("settings").show(gui, |arg_ui| {
        _ = arg_ui.label("Line width");
        _ = arg_ui.add(
            egui::DragValue::new(line_width)
                .speed(0.02)
                .range(0.5..=20.0),
        );
        _ = arg_ui.horizontal_top(|ui| {
            axis_cfg.toggle_axis_cfg_ui(ui);
            _ = ui.label("|");
            plot_visibility_cfg.toggle_visibility_ui(ui);
        });

        _ = arg_ui.horizontal_centered(|ui| {
            _ = ui.label("| ");
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

            _ = ui.label(RichText::new(play_state.formatted_time()));
            _ = ui.label(" |");
        });

        arg_ui.end_row();
    });
}
