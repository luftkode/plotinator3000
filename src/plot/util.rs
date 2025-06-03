use egui::{Modifiers, Vec2};
use skytem_plot_util::{Plots, StoredPlotLabels};
use skytem_log_if::prelude::*;

use crate::app::supported_formats::SupportedFormat;

use super::plot_settings::{PlotSettings, date_settings::LoadedLogSettings};

pub fn add_plot_data_to_plot_collections(
    plots: &mut Plots,
    data: &SupportedFormat,
    plot_settings: &mut PlotSettings,
) {
    // This is how all logs get their log_id, and how each plot for each log gets their log_id
    let data_id = plot_settings.next_log_id();

    plot_settings.add_log_setting(LoadedLogSettings::new(
        data_id,
        data.descriptive_name().to_owned(),
        data.first_timestamp(),
        data.metadata(),
        data.parse_info(),
    ));
    for raw_plot in data.raw_plots() {
        match raw_plot.expected_range() {
            ExpectedPlotRange::Percentage => {
                plots
                    .percentage_mut()
                    .add_plot_if_not_exists(raw_plot, data_id);
            }
            ExpectedPlotRange::OneToOneHundred => {
                plots
                    .one_to_hundred_mut()
                    .add_plot_if_not_exists(raw_plot, data_id);
            }
            ExpectedPlotRange::Thousands => {
                plots
                    .thousands_mut()
                    .add_plot_if_not_exists(raw_plot, data_id);
            }
        }
        plot_settings.add_plot_name_if_not_exists(raw_plot.name());
    }

    if let Some(plot_labels) = data.labels() {
        for labels in plot_labels {
            let owned_label_points = labels.label_points().to_owned();
            match labels.expected_range() {
                ExpectedPlotRange::Percentage => plots
                    .percentage_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, data_id)),
                ExpectedPlotRange::OneToOneHundred => {
                    plots
                        .one_to_hundred_mut()
                        .add_plot_labels(StoredPlotLabels::new(owned_label_points, data_id));
                }
                ExpectedPlotRange::Thousands => plots
                    .thousands_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, data_id)),
            }
        }
    }
}

/// Returns input scroll and modifiers for a given UI element.
pub fn get_cursor_scroll_input(ui: &egui::Ui) -> (Option<Vec2>, Modifiers) {
    ui.input(|i| {
        let scroll = i.events.iter().find_map(|e| match e {
            egui::Event::MouseWheel {
                unit: _,
                delta,
                modifiers: _,
            } => Some(*delta),
            _ => None,
        });
        (scroll, i.modifiers)
    })
}

/// Set and return a zoom factor from input scroll and modifiers.
///
/// * `CTRL` + scroll: Zoom on X-axis
/// * `CTRL + ALT`: Zoom on Y-axis
pub fn set_zoom_factor(scroll: Vec2, modifiers: Modifiers) -> Option<Vec2> {
    const SCROLL_MULTIPLIER: f32 = 0.1;
    let scroll = Vec2::splat(scroll.x + scroll.y);
    let mut zoom_factor = Vec2::from([
        (scroll.x * SCROLL_MULTIPLIER).exp(),
        (scroll.y * SCROLL_MULTIPLIER).exp(),
    ]);

    let ctrl = modifiers.ctrl;
    let ctrl_plus_alt = modifiers.alt && ctrl;

    if ctrl_plus_alt {
        log::debug!("CTRL+ALT locks X-axis");
        zoom_factor.x = 1.0;
    } else if ctrl {
        log::debug!("CTRL locks Y-axis");
        zoom_factor.y = 1.0;
    }

    log::debug!("zoom_factor={zoom_factor}");
    if ctrl { Some(zoom_factor) } else { None }
}
