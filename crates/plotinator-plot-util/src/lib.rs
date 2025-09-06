pub mod mipmap;

use std::ops::RangeInclusive;

use egui::Color32;
use egui_plot::{PlotPoint, PlotPoints};

pub mod draw_series;
pub(crate) mod filter;
pub mod plots;

pub use plots::{
    Plots,
    plot_data::{PlotData, PlotValues, plot_labels::StoredPlotLabels},
};

use crate::draw_series::SeriesDrawMode;

/// An instance of a `MipMap` configuration for a given frame
#[derive(Debug, Clone, Copy)]
pub enum MipMapConfiguration {
    Manual(usize),
    Auto,
    Disabled,
}

pub fn plot_lines<'pv>(
    plot_ui: &mut egui_plot::PlotUi<'pv>,
    plots: impl Iterator<Item = &'pv PlotValues>,
    mipmap_cfg: MipMapConfiguration,
    series_draw_mode: SeriesDrawMode,
    plots_width_pixels: usize,
) {
    plotinator_macros::profile_function!();
    let x_plot_bounds = plot_ui.plot_bounds().range_x();
    for plot_vals in plots {
        match mipmap_cfg {
            MipMapConfiguration::Disabled => {
                plot_raw(plot_ui, plot_vals, series_draw_mode, x_plot_bounds.clone());
            }
            MipMapConfiguration::Auto => {
                let (level, idx_range) =
                    plot_vals.get_scaled_mipmap_levels(plots_width_pixels, x_plot_bounds.clone());

                plot_with_mipmapping(
                    plot_ui,
                    plot_vals,
                    series_draw_mode,
                    level,
                    x_plot_bounds.clone(),
                    idx_range,
                );
            }
            MipMapConfiguration::Manual(level) => {
                plot_with_mipmapping(
                    plot_ui,
                    plot_vals,
                    series_draw_mode,
                    level,
                    x_plot_bounds.clone(),
                    None,
                );
            }
        }
    }
}

fn plot_with_mipmapping<'p>(
    plot_ui: &mut egui_plot::PlotUi<'p>,
    plot_vals: &'p PlotValues,

    series_draw_mode: SeriesDrawMode,
    mipmap_lvl: usize,
    x_bounds: RangeInclusive<f64>,
    // if the range is already known then we can skip filtering
    known_idx_range: Option<(usize, usize)>,
) {
    plotinator_macros::profile_function!();

    let plot_points_minmax = plot_vals.get_level_or_max(mipmap_lvl);
    if plot_points_minmax.is_empty() {
        // In this case there was so few samples that downsampling just once was below the minimum threshold, so we just plot all samples
        plot_raw(plot_ui, plot_vals, series_draw_mode, x_bounds);
    } else {
        let plot_points_minmax = match known_idx_range {
            Some((start, end)) => PlotPoints::Borrowed(&plot_points_minmax[start..end]),
            None => filter::filter_plot_points(plot_points_minmax, x_bounds),
        };

        series_draw_mode.draw_series(
            plot_ui,
            plot_points_minmax,
            plot_vals.label(),
            plot_vals.get_color(),
            plot_vals.get_highlight(),
        );
    }
}

fn plot_raw<'p>(
    plot_ui: &mut egui_plot::PlotUi<'p>,
    plot_vals: &'p PlotValues,

    series_draw_mode: SeriesDrawMode,
    x_bounds: RangeInclusive<f64>,
) {
    plotinator_macros::profile_function!();
    let plot_points = plot_vals.raw_plot_points();
    let filtered_points = filter::filter_plot_points(plot_points, x_bounds);
    series_draw_mode.draw_series(
        plot_ui,
        filtered_points,
        plot_vals.label(),
        plot_vals.get_color(),
        plot_vals.get_highlight(),
    );
}

pub fn plot_raw_mqtt_line<'p>(
    plot_ui: &mut egui_plot::PlotUi<'p>,
    label: &str,
    plot_points: &'p [PlotPoint],
    color: Color32,
    series_draw_mode: SeriesDrawMode,
    x_bounds: RangeInclusive<f64>,
) {
    plotinator_macros::profile_function!();

    let filtered_points = filter::filter_plot_points(plot_points, x_bounds);
    series_draw_mode.draw_series(plot_ui, filtered_points, label, color, false);
}

pub fn plot_labels(plot_ui: &mut egui_plot::PlotUi, plot_data: &PlotData, id_filter: &[u16]) {
    plotinator_macros::profile_function!();
    for plot_labels in plot_data
        .plot_labels()
        .iter()
        .filter(|pl| !id_filter.contains(&pl.log_id))
    {
        for label in plot_labels.labels() {
            let point = PlotPoint::new(label.point()[0], label.point()[1]);
            let mut txt = egui::RichText::new(label.text()).size(10.0);
            if plot_labels.get_highlight() {
                txt = txt.strong();
            }
            let txt = egui_plot::Text::new("", point, txt);
            plot_ui.text(txt);
        }
    }
}
