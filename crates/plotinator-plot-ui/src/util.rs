use egui::{Modifiers, Vec2};
use plotinator_log_if::prelude::*;
use plotinator_plot_util::{CookedPlot, Plots, StoredPlotLabels};
use plotinator_supported_formats::SupportedFormat;
use plotinator_ui_util::ExpectedPlotRange;
use rayon::prelude::*;

use super::plot_settings::{PlotSettings, date_settings::LoadedLogSettings};

#[plotinator_proc_macros::log_time]
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

    const PARALLEL_THRESHOLD: usize = 200_000;
    // We just check the first one, usually formats will have the same number of points for all the data series
    let first_plot_points_count: usize = data.raw_plots().first().map_or(0, |p| match p {
        RawPlot::Generic { common } => common.points().len(),
        RawPlot::GeoSpatialDataset(geo) => geo.len(),
    });

    if first_plot_points_count > PARALLEL_THRESHOLD {
        log::info!(
            "Processing new plots in parallel (point count {first_plot_points_count} exceeds threshold {PARALLEL_THRESHOLD})"
        );
        add_plot_points_to_collections_par(plots, data, data_id);
    } else {
        add_plot_points_to_collections_seq(plots, data, data_id);
    }

    for raw_plot in data.raw_plots() {
        match raw_plot {
            RawPlot::GeoSpatialDataset(geo_data) => {
                for common in geo_data.raw_plots_common() {
                    plot_settings.add_plot_name_if_not_exists(
                        common.ty().to_owned(),
                        data.descriptive_name(),
                    );
                }
            }
            RawPlot::Generic { common } => {
                plot_settings
                    .add_plot_name_if_not_exists(common.ty().to_owned(), data.descriptive_name());
            }
        }
    }

    add_plot_labels_to_collections(plots, data, data_id);
}

fn add_plot_points_to_collections_par(plots: &mut Plots, data: &SupportedFormat, data_id: u16) {
    let existing_plots_percentage: Vec<&str> = plots.percentage().plot_labels_iter().collect();
    let existing_plots_one_to_hundred: Vec<&str> =
        plots.one_to_hundred().plot_labels_iter().collect();
    let existing_plots_thousands: Vec<&str> = plots.thousands().plot_labels_iter().collect();

    // Extract all GeoSpatial plots into owned Vec
    let geo_plots: Vec<RawPlotCommon> = data
        .raw_plots()
        .iter()
        .filter_map(|rp| match rp {
            RawPlot::GeoSpatialDataset(geo_data) => Some(geo_data.raw_plots_common()),
            RawPlot::Generic { .. } => None,
        })
        .flatten()
        .collect();

    // Process all plots in parallel: owned geo plots + borrowed other plots
    let new_cooked_plots: Vec<(ExpectedPlotRange, CookedPlot)> = geo_plots
        .par_iter()
        .chain(data.raw_plots().par_iter().filter_map(|rp| match rp {
            RawPlot::Generic { common } => Some(common),
            RawPlot::GeoSpatialDataset(_) => None,
        }))
        .filter_map(|rpc| {
            let label = rpc.label_from_id(data_id);
            let already_exists = match rpc.expected_range() {
                ExpectedPlotRange::Percentage => {
                    existing_plots_percentage.contains(&label.as_str())
                }
                ExpectedPlotRange::Hundreds => {
                    existing_plots_one_to_hundred.contains(&label.as_str())
                }
                ExpectedPlotRange::Thousands => existing_plots_thousands.contains(&label.as_str()),
            };

            if already_exists {
                None
            } else {
                let cooked_plot = CookedPlot::new(rpc, data_id, data.descriptive_name().to_owned());
                Some((rpc.expected_range(), cooked_plot))
            }
        })
        .collect();

    // Add the newly created plots to their respective collections sequentially.
    for (range, new_plot) in new_cooked_plots {
        match range {
            ExpectedPlotRange::Percentage => {
                plots.percentage_mut().plots_as_mut().push(new_plot);
            }
            ExpectedPlotRange::Hundreds => {
                plots.one_to_hundred_mut().plots_as_mut().push(new_plot);
            }
            ExpectedPlotRange::Thousands => {
                plots.thousands_mut().plots_as_mut().push(new_plot);
            }
        }
    }

    // Recalculate the max bounds for each collection once after all plots are added.
    plots.percentage_mut().calc_max_bounds();
    plots.one_to_hundred_mut().calc_max_bounds();
    plots.thousands_mut().calc_max_bounds();
}

fn add_plot_points_to_collections_seq(plots: &mut Plots, data: &SupportedFormat, data_id: u16) {
    for raw_plot in data.raw_plots() {
        match raw_plot {
            RawPlot::Generic { common } => match common.expected_range() {
                ExpectedPlotRange::Percentage => {
                    plots.percentage_mut().add_plot_if_not_exists(
                        common,
                        data_id,
                        data.descriptive_name(),
                    );
                }
                ExpectedPlotRange::Hundreds => {
                    plots.one_to_hundred_mut().add_plot_if_not_exists(
                        common,
                        data_id,
                        data.descriptive_name(),
                    );
                }
                ExpectedPlotRange::Thousands => {
                    plots.thousands_mut().add_plot_if_not_exists(
                        common,
                        data_id,
                        data.descriptive_name(),
                    );
                }
            },
            RawPlot::GeoSpatialDataset(geo_data) => {
                for common in geo_data.raw_plots_common() {
                    match common.expected_range() {
                        ExpectedPlotRange::Percentage => {
                            plots.percentage_mut().add_plot_if_not_exists(
                                &common,
                                data_id,
                                data.descriptive_name(),
                            );
                        }
                        ExpectedPlotRange::Hundreds => {
                            plots.one_to_hundred_mut().add_plot_if_not_exists(
                                &common,
                                data_id,
                                data.descriptive_name(),
                            );
                        }
                        ExpectedPlotRange::Thousands => {
                            plots.thousands_mut().add_plot_if_not_exists(
                                &common,
                                data_id,
                                data.descriptive_name(),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn add_plot_labels_to_collections(plots: &mut Plots, data: &SupportedFormat, data_id: u16) {
    if let Some(plot_labels) = data.labels() {
        for labels in plot_labels {
            let owned_label_points = labels.label_points().to_owned();
            match labels.expected_range() {
                ExpectedPlotRange::Percentage => plots
                    .percentage_mut()
                    .add_plot_labels(StoredPlotLabels::new(owned_label_points, data_id)),
                ExpectedPlotRange::Hundreds => {
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
