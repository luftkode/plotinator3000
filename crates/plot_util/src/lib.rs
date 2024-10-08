pub mod mipmap;

use egui_plot::{Line, PlotBounds, PlotItem, PlotPoints};
use log_if::prelude::*;
use mipmap::MipMap1D;
use serde::{Deserialize, Serialize};

pub mod plots;

pub use plots::{
    plot_data::{PlotData, PlotValues, StoredPlotLabels},
    Plots,
};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct MipMapWithName {
    pub mip_map: MipMap1D<f64>,
    pub name: String,
}

impl MipMapWithName {
    pub fn new(raw_y: Vec<f64>, name: String) -> Self {
        let mip_map = MipMap1D::new(raw_y);
        Self { mip_map, name }
    }
}

pub fn line_from_log_entry<XF, YF, L: LogEntry>(log: &[L], x_extractor: XF, y_extractor: YF) -> Line
where
    XF: Fn(&L) -> f64,
    YF: Fn(&L) -> f64,
{
    let points: PlotPoints = log
        .iter()
        .map(|e| [x_extractor(e), y_extractor(e)])
        .collect();
    Line::new(points)
}

pub fn plot_lines(
    plot_ui: &mut egui_plot::PlotUi,
    plots: &[PlotValues],
    name_filter: &[&str],
    id_filter: &[usize],
    line_width: f32,
    mipmap_lvl: usize,
    plots_width_pixels: usize,
) {
    for plot_vals in plots
        .iter()
        .filter(|p| !name_filter.contains(&p.name()) && !id_filter.contains(&p.log_id()))
    {
        let (x_min, x_max) = x_plot_bound(plot_ui.plot_bounds());
        let x_bounds = (x_min as usize, x_max as usize);
        let scaled_lvl = plot_vals.get_scaled_mipmap_levels(plots_width_pixels, x_bounds);
        let (plot_points_min, plot_points_max) =
            plot_vals.get_level(scaled_lvl).unwrap_or_else(|| {
                plot_vals
                    .get_level(plot_vals.mipmap_levels() - 1)
                    .expect("Logic error")
            });
        let x_min_max_ext = extended_x_plot_bound(plot_ui.plot_bounds(), 0.1);
        let filtered_points_min = filter_plot_points(plot_points_min, x_min_max_ext);
        let filtered_points_max = filter_plot_points(plot_points_max, x_min_max_ext);

        // Manual string construction for efficiency since this is a hot path.
        let mut label_min = plot_vals.label().to_owned();
        label_min.push_str(" [min]");
        let mut label_max = plot_vals.label().to_owned();
        label_max.push_str(" [max]");
        // TODO: Make some kind of rotating color scheme such that min/max plots look kind of similar but that a lot of different colors are still used
        let line_min = Line::new(filtered_points_min).name(label_min);
        let line_max = Line::new(filtered_points_max).name(label_max);
        plot_ui.line(line_min.width(line_width));
        plot_ui.line(line_max.width(line_width));
    }
}

fn x_plot_bound(bounds: PlotBounds) -> (f64, f64) {
    let range = bounds.range_x();
    (*range.start(), *range.end())
}

/// Extends the x plot bounds by a specified percentage in both directions
pub fn extended_x_plot_bound(bounds: PlotBounds, extension_percentage: f64) -> (f64, f64) {
    let (x_bound_min, x_bound_max) = x_plot_bound(bounds);

    // Calculate the extension values based on the magnitude of the bounds
    let x_extension = (x_bound_max - x_bound_min).abs() * extension_percentage;

    // Extend the bounds
    let extended_x_bound_min = x_bound_min - x_extension;
    let extended_x_bound_max = x_bound_max + x_extension;

    (extended_x_bound_min, extended_x_bound_max)
}

#[inline(always)]
fn point_within(point: f64, bounds: (f64, f64)) -> bool {
    let (min, max) = bounds;
    min < point && point < max
}

/// Filter plot points based on the x plot bounds. Always includes the first and last plot point
/// such that resetting zooms works well even when the plot bounds are outside the data range.
pub fn filter_plot_points(points: &[[f64; 2]], x_range: (f64, f64)) -> Vec<[f64; 2]> {
    if points.is_empty() {
        return Vec::new();
    } else if points.len() < 3 {
        return points.to_owned();
    }

    let mut filtered = Vec::with_capacity(points.len());

    // Always include the first point
    filtered.push(points[0]);

    // Filter points within the extended range
    filtered.extend(
        points
            .iter()
            .skip(1)
            .take(points.len() - 2)
            .filter(|point| point_within(point[0], x_range))
            .copied(),
    );

    // Always include the last point if it's different from the first point
    if let Some(last_point) = points.last() {
        if *last_point != filtered[0] {
            filtered.push(*last_point);
        }
    }

    filtered
}
