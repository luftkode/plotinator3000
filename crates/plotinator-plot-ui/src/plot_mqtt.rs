use egui::{Color32, Vec2};
use egui_plot::{Plot, PlotBounds};
use plotinator_mqtt_ui::plot::MqttPlotPoints;
use plotinator_plot_util::draw_series::SeriesDrawMode;
use plotinator_ui_util::{PlotType, box_selection::BoxSelection};

use crate::util;

use super::click_delta::ClickDelta;

/// Iterates through and fills/paints all plots with their respective data.
#[allow(clippy::too_many_arguments)]
pub fn fill_mqtt_plots(
    gui: &mut egui::Ui,
    reset_plot_bounds: bool,
    series_draw_mode: SeriesDrawMode,
    click_delta: &mut ClickDelta,
    mqtt_plot_area: Plot<'_>,
    mqtt_plots: &[(MqttPlotPoints, Color32)],
    set_auto_bounds: &mut bool,
    box_selection: &mut BoxSelection,
) {
    plotinator_macros::profile_function!();

    let (scroll, modifiers) = util::get_cursor_scroll_input(gui);
    let final_zoom_factor: Option<Vec2> = scroll.and_then(|s| util::set_zoom_factor(s, modifiers));

    mqtt_plot_area.show(gui, |plot_ui| {
        let area_hovered = plot_ui.response().hovered();
        if area_hovered {
            box_selection.record_key_and_pointer_events(plot_ui, PlotType::Hundreds);
        }
        if area_hovered && let Some(final_zoom_factor) = final_zoom_factor {
            plot_ui.zoom_bounds_around_hovered(final_zoom_factor);
        }
        let resp = plot_ui.response();
        if plot_ui.response().double_clicked() || reset_plot_bounds {
            *set_auto_bounds = true;
            if let Some(max_bounds) = get_mqtt_auto_scaled_plot_bounds(mqtt_plots) {
                plot_ui.set_plot_bounds(max_bounds);
            }
        } else if resp.clicked() {
            if plot_ui.ctx().input(|i| i.modifiers.shift) {
                if let Some(pointer_coordinate) = plot_ui.pointer_coordinate() {
                    click_delta.set_next_click(pointer_coordinate, PlotType::Hundreds);
                }
            } else {
                click_delta.reset();
            }
        } else if *set_auto_bounds {
            plot_ui.set_auto_bounds(true);
            *set_auto_bounds = false;
        }

        click_delta.ui(plot_ui, PlotType::Hundreds);
        let x_bounds = plot_ui.plot_bounds().range_x();
        for (mp, color) in mqtt_plots {
            if mp.data.len() < 2 {
                // We don't plot less than two points. It's mostly because when the
                // plotting starts, the auto-bounds causes a crash due to auto sizing
                // a plot to 1 point and triggering an assert in egui_plot that the
                // height and width of the bounds is greater than 0.0
                continue;
            }
            plotinator_plot_util::plot_raw_mqtt_line(
                plot_ui,
                &mp.topic,
                &mp.data,
                *color,
                series_draw_mode,
                x_bounds.clone(),
            );
        }
    });
}

pub fn get_mqtt_auto_scaled_plot_bounds(
    mqtt_plots: &[(MqttPlotPoints, Color32)],
) -> Option<PlotBounds> {
    let mut max_bounds: Option<PlotBounds> = None;
    for (mp, _) in mqtt_plots {
        let mp_first_point = mp
            .data
            .first()
            .expect("Invalid empty MQTT plot data vector");
        let tmp_bounds = if mp.data.len() < 2 {
            PlotBounds::from_min_max(
                [mp_first_point.x, mp_first_point.y],
                [mp_first_point.x, mp_first_point.y],
            )
        } else {
            let min_x = mp_first_point.x;
            let max_x = mp
                .data
                .last()
                .expect("Should be unreachable: Invalid empty MQTT plot data vector")
                .x;
            let mut min_y = mp_first_point.y;
            let mut max_y = mp_first_point.y;
            for p in &mp.data {
                if p.y < min_y {
                    min_y = p.y;
                }
                if p.y > max_y {
                    max_y = p.y;
                }
            }
            PlotBounds::from_min_max([min_x, min_y], [max_x, max_y])
        };
        if let Some(max_bounds) = &mut max_bounds {
            max_bounds.merge(&tmp_bounds);
        } else {
            max_bounds = Some(tmp_bounds);
        }
    }
    if let Some(mut max_bounds) = max_bounds {
        // finally extend each bound by 10%
        let margin_fraction = egui::Vec2::splat(0.1);
        max_bounds.add_relative_margin_x(margin_fraction);
        max_bounds.add_relative_margin_y(margin_fraction);
        if max_bounds.is_valid() {
            return Some(max_bounds);
        } else if max_bounds.is_valid_x() {
            // Could happen if all points (y-value) are identical, so we just expand y and then we can use the bounds
            max_bounds.expand_y(1.0);
            return Some(max_bounds);
        }
    }
    None
}
