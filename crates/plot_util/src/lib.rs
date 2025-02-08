pub mod mipmap;

use std::slice;

use egui_plot::{Line, PlotBounds, PlotPoint, PlotPoints};

pub mod plots;

pub use plots::{
    plot_data::{PlotData, PlotValues, StoredPlotLabels},
    Plots,
};

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
    line_width: f32,
    mipmap_cfg: MipMapConfiguration,
    plots_width_pixels: usize,
) {
    let (x_lower, x_higher) = extended_x_plot_bound(plot_ui.plot_bounds(), 0.1);
    for plot_vals in plots {
        match mipmap_cfg {
            MipMapConfiguration::Disabled => {
                plot_raw(plot_ui, plot_vals, line_width, (x_lower, x_higher));
            }
            MipMapConfiguration::Auto => {
                let (level, idx_range) =
                    plot_vals.get_scaled_mipmap_levels(plots_width_pixels, (x_lower, x_higher));

                plot_with_mipmapping(
                    plot_ui,
                    plot_vals,
                    line_width,
                    level,
                    (x_lower, x_higher),
                    idx_range,
                );
            }
            MipMapConfiguration::Manual(level) => {
                plot_with_mipmapping(
                    plot_ui,
                    plot_vals,
                    line_width,
                    level,
                    (x_lower, x_higher),
                    None,
                );
            }
        }
    }
}

fn plot_with_mipmapping<'p>(
    plot_ui: &mut egui_plot::PlotUi<'p>,
    plot_vals: &'p PlotValues,
    line_width: f32,
    mipmap_lvl: usize,
    x_range: (f64, f64),
    // if the range is already known then we can skip filtering
    known_idx_range: Option<(usize, usize)>,
) {
    let (x_lower, x_higher) = x_range;
    // If the mipmap level is 0 or 1 plotting all data points is just as efficient.
    if mipmap_lvl < 2 {
        plot_raw(plot_ui, plot_vals, line_width, (x_lower, x_higher));
    } else {
        let plot_points_minmax = plot_vals.get_level_or_max(mipmap_lvl);
        if plot_points_minmax.is_empty() {
            // In this case there was so few samples that downsampling just once was below the minimum threshold, so we just plot all samples
            plot_raw(plot_ui, plot_vals, line_width, (x_lower, x_higher));
        } else {
            let plot_points_minmax = match known_idx_range {
                Some((start, end)) => extract_range_points_alt(plot_points_minmax, start, end),
                None => filter_plot_points_alt(plot_points_minmax, (x_lower, x_higher)),
            };

            let line = Line::new(plot_points_minmax)
                .name(plot_vals.label())
                .color(plot_vals.get_color())
                .highlight(plot_vals.get_highlight());

            plot_ui.line(line.width(line_width));
        }
    }
}

#[inline(always)]
fn extract_range_points(points: &[[f64; 2]], start: usize, end: usize) -> Vec<[f64; 2]> {
    let element_count = end - start + 2;
    let mut final_points = Vec::with_capacity(element_count);
    final_points.push(points[0]);

    final_points.extend_from_slice(&points[start..end]);

    if let Some(last_point) = points.last() {
        if points.last().is_some_and(|lp| lp != last_point) {
            final_points.push(*last_point);
        }
    }

    final_points
}

#[inline(always)]
fn extract_range_points_alt<'a>(
    points: &'a [PlotPoint],
    start: usize,
    end: usize,
) -> PlotPoints<'a> {
    let element_count = end - start + 2;
    let mut final_points = Vec::with_capacity(element_count);
    final_points.push(points[0]);

    final_points.extend_from_slice(&points[start..end]);

    if let Some(last_point) = points.last() {
        if points.last().is_some_and(|lp| lp != last_point) {
            final_points.push(*last_point);
        }
    }

    PlotPoints::Owned(final_points)
}

pub fn plot_labels(plot_ui: &mut egui_plot::PlotUi, plot_data: &PlotData, id_filter: &[u16]) {
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
            let txt = egui_plot::Text::new(point, txt);
            plot_ui.text(txt);
        }
    }
}

fn plot_raw<'p>(
    plot_ui: &mut egui_plot::PlotUi<'p>,
    plot_vals: &'p PlotValues,
    line_width: f32,
    x_min_max_ext: (f64, f64),
) {
    let plot_points = plot_vals.raw_plot_points();

    let filtered_points = filter_plot_points_alt(plot_points, x_min_max_ext);
    let line = Line::new(filtered_points)
        .width(line_width)
        .name(plot_vals.label())
        .color(plot_vals.get_color())
        .highlight(plot_vals.get_highlight());
    plot_ui.line(line);
}

#[inline(always)]
fn x_plot_bound(bounds: PlotBounds) -> (f64, f64) {
    let range = bounds.range_x();
    (*range.start(), *range.end())
}

/// Extends the x plot bounds by a specified percentage in both directions
#[inline]
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
pub fn filter_plot_points_alt<'a>(points: &'a [PlotPoint], x_range: (f64, f64)) -> PlotPoints<'a> {
    let points_len = points.len();

    // Don't bother filtering if there's less than 1024 points
    if points_len < 1024 {
        return PlotPoints::Borrowed(points); // Borrow if no filtering is needed
    }

    let start_idx = points.partition_point(|point| point.x < x_range.0);
    let end_idx = points.partition_point(|point| point.x < x_range.1);

    let points_within = end_idx - start_idx;

    // If all the points are within the bound, return all the points
    if points_within == points_len {
        return PlotPoints::Borrowed(points); // Borrow if no filtering is needed
    }

    // In this case none of the points are within the bounds so just return the first and last
    if start_idx == end_idx {
        return PlotPoints::Owned(vec![points[0], points[points_len - 1]]); // Copy only first/last
    }

    // allocate enough for the points within + 2 for the first and last points.
    let mut filtered = Vec::with_capacity(points_within + 2);

    // add the first points if it is not within the points that are within the bounds
    if start_idx != 0 {
        filtered.push(points[0]);
    }
    // Add all the points within the bounds
    filtered.extend_from_slice(&points[start_idx..end_idx]);

    // add the last points if it is not included in the points that are within the bounds
    if end_idx != points_len {
        filtered.push(points[points_len - 1]);
    }

    PlotPoints::Owned(filtered) // Copy the filtered data
}

/// Filter plot points based on the x plot bounds. Always includes the first and last plot point
/// such that resetting zooms works well even when the plot bounds are outside the data range.
pub fn filter_plot_points(points: &[[f64; 2]], x_range: (f64, f64)) -> Vec<[f64; 2]> {
    let points_len = points.len();
    // Don't bother filtering if there's less than 1024 points
    if points_len < 1024 {
        return points.to_vec();
    }

    let start_idx = points.partition_point(|point| point[0] < x_range.0);
    let end_idx = points.partition_point(|point| point[0] < x_range.1);

    let points_within = end_idx - start_idx;
    // If all the points are within the bound, return all the points
    if points_within == points_len {
        return points.to_vec();
    }
    // In this case none of the points are within the bounds so just return the first and last
    if start_idx == end_idx {
        return vec![points[0], points[points_len - 1]];
    }

    // allocate enough for the points within + 2 for the first and last points.
    // we might not end up including the first and last points if they are included in the points within
    // but this way we are sure to only allocate once
    let mut filtered = Vec::with_capacity(points_within + 2);

    // add the first points if it is not within the points that are within the bounds
    if start_idx != 0 {
        filtered.push(points[0]);
    }
    // Add all the points within the bounds
    filtered.extend_from_slice(&points[start_idx..end_idx]);

    // add the last points if it is not included in the points that are within the bounds
    if end_idx != points_len {
        filtered.push(points[points_len - 1]);
    }

    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_less_than_1024_points_no_filtering() {
        let points: Vec<[f64; 2]> = (0..500).map(|i| [i as f64, i as f64 + 1.0]).collect();
        let x_range = (100.0, 300.0);

        // Since points are less than 1024, no filtering should be done
        let result = filter_plot_points(&points, x_range);

        // Result should be identical to input
        assert_eq!(result, points);
    }

    #[test]
    fn test_more_than_1024_points_with_filtering() {
        let points: Vec<[f64; 2]> = (0..1500).map(|i| [i as f64, i as f64 + 1.0]).collect();
        let x_range = (100.0, 500.0);

        // Since the points are more than 1024, filtering should happen
        let result = filter_plot_points(&points, x_range);

        // First point, range of points between start and end range, last point should be included
        let mut expected: Vec<[f64; 2]> = vec![
            // First point
            [0.0, 1.0],
        ];
        // Points within the range (100..500)
        expected.extend_from_slice(&points[100..500]);
        // Last point
        expected.push([1499.0, 1500.0]);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_range_outside_bounds_with_large_data() {
        let points: Vec<[f64; 2]> = (0..1500).map(|i| [i as f64, i as f64 + 1.0]).collect();
        let x_range = (2000.0, 3000.0);

        // Since range is outside the data points, we should get first and last points
        let result = filter_plot_points(&points, x_range);

        let expected = vec![[0.0, 1.0], [1499.0, 1500.0]];

        assert_eq!(result, expected);
    }
}
