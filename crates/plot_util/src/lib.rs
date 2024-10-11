pub mod mipmap;

use egui_plot::{Line, PlotBounds, PlotPoint, PlotPoints};
use log_if::prelude::*;

pub mod plots;

pub use plots::{
    plot_data::{PlotData, PlotValues, StoredPlotLabels},
    Plots,
};

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

/// An instance of a `MipMap` configuration for a given frame
#[derive(Debug, Clone, Copy)]
pub enum MipMapConfiguration {
    Enabled(Option<usize>),
    Disabled,
}

pub fn plot_lines(
    plot_ui: &mut egui_plot::PlotUi,
    plots: &mut [PlotValues],
    name_filter: &[&str],
    id_filter: &[usize],
    line_width: f32,
    mipmap_cfg: MipMapConfiguration,
    plots_width_pixels: usize,
) {
    let x_min_max_ext = extended_x_plot_bound(plot_ui.plot_bounds(), 0.1);
    for plot_vals in plots
        .iter_mut()
        .filter(|p| !name_filter.contains(&p.name()) && !id_filter.contains(&p.log_id()))
    {
        // TODO: Make some kind of rotating color scheme such that min/max plots look kind of similar but that a lot of different colors are still used
        match mipmap_cfg {
            MipMapConfiguration::Disabled => plot_raw(plot_ui, plot_vals, x_min_max_ext),
            MipMapConfiguration::Enabled(level_option) => {
                let (level, idx_range) = match level_option {
                    Some(lvl) => (lvl, None),
                    None => plot_vals.get_scaled_mipmap_levels(
                        plots_width_pixels,
                        (x_min_max_ext.0 as usize, x_min_max_ext.1 as usize),
                    ),
                };

                if level == 0 {
                    plot_raw(plot_ui, plot_vals, x_min_max_ext);
                    continue;
                }

                let (plot_points_min, plot_points_max) = plot_vals.get_level_or_max(level);
                if plot_points_min.is_empty() {
                    continue;
                }

                let points = match idx_range {
                    Some((start, end)) => {
                        extract_range_points(plot_points_min, plot_points_max, start, end)
                    }
                    None => (
                        filter_plot_points(plot_points_min, x_min_max_ext),
                        filter_plot_points(plot_points_max, x_min_max_ext),
                    ),
                };

                plot_min_max_lines(plot_ui, plot_vals.label(), points, line_width);
            }
        }
    }
}

#[inline(always)]
fn extract_range_points(
    points_min: &[[f64; 2]],
    points_max: &[[f64; 2]],
    start: usize,
    end: usize,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let element_count = end - start + 2;
    let mut min_points = Vec::with_capacity(element_count);
    let mut max_points = Vec::with_capacity(element_count);

    min_points.push(points_min[0]);
    max_points.push(points_max[0]);

    min_points.extend_from_slice(&points_min[start..end]);
    max_points.extend_from_slice(&points_max[start..end]);

    min_points.push(*points_min.last().unwrap());
    max_points.push(*points_max.last().unwrap());

    (min_points, max_points)
}

#[inline]
fn plot_min_max_lines(
    plot_ui: &mut egui_plot::PlotUi,
    base_label: &str,
    (points_min, points_max): (Vec<[f64; 2]>, Vec<[f64; 2]>),
    line_width: f32,
) {
    let mut label_min = base_label.to_owned();
    label_min.push_str(" (min)");
    let mut label_max = base_label.to_owned();
    label_max.push_str(" (max)");

    let line_min = Line::new(points_min).name(label_min);
    let line_max = Line::new(points_max).name(label_max);

    plot_ui.line(line_min.width(line_width));
    plot_ui.line(line_max.width(line_width));
}

pub fn plot_labels(plot_ui: &mut egui_plot::PlotUi, plot_data: &PlotData, id_filter: &[usize]) {
    for plot_labels in plot_data
        .plot_labels()
        .iter()
        .filter(|pl| !id_filter.contains(&pl.log_id))
    {
        for label in plot_labels.labels() {
            let point = PlotPoint::new(label.point()[0], label.point()[1]);
            let txt = egui::RichText::new(label.text()).size(10.0);
            let txt = egui_plot::Text::new(point, txt);
            plot_ui.text(txt);
        }
    }
}

fn plot_raw(plot_ui: &mut egui_plot::PlotUi, plot_vals: &PlotValues, x_min_max_ext: (f64, f64)) {
    let plot_points = plot_vals.get_raw();
    let filtered_points = filter_plot_points(plot_points, x_min_max_ext);
    let line = Line::new(filtered_points).name(plot_vals.label());
    plot_ui.line(line);
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

/// Filter plot points based on the x plot bounds. Always includes the first and last plot point
/// such that resetting zooms works well even when the plot bounds are outside the data range.
pub fn filter_plot_points(points: &[[f64; 2]], x_range: (f64, f64)) -> Vec<[f64; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut filtered = Vec::with_capacity(points.len());

    // Always include the first point
    filtered.push(points[0]);

    // Find start index
    let start_idx = points
        .partition_point(|point| point[0] < x_range.0)
        .saturating_sub(1);

    // Find end index
    let end_idx = points.partition_point(|point| point[0] <= x_range.1);

    // Add points within range
    filtered.extend_from_slice(&points[start_idx..end_idx]);

    // Add last point if different from first
    let last_point = points[points.len() - 1];
    if last_point != filtered[0] {
        filtered.push(last_point);
    }

    filtered
}
