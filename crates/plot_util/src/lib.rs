pub mod mipmap;

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
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();
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
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();
    let (x_lower, x_higher) = x_range;

    let plot_points_minmax = plot_vals.get_level_or_max(mipmap_lvl);
    if plot_points_minmax.is_empty() {
        // In this case there was so few samples that downsampling just once was below the minimum threshold, so we just plot all samples
        plot_raw(plot_ui, plot_vals, line_width, (x_lower, x_higher));
    } else {
        let plot_points_minmax = match known_idx_range {
            Some((start, end)) => PlotPoints::Borrowed(&plot_points_minmax[start..end]),
            None => filter_plot_points(plot_points_minmax, (x_lower, x_higher)),
        };

        let line = Line::new(plot_points_minmax)
            .name(plot_vals.label())
            .color(plot_vals.get_color())
            .highlight(plot_vals.get_highlight());

        plot_ui.line(line.width(line_width));
    }
}

pub fn plot_labels(plot_ui: &mut egui_plot::PlotUi, plot_data: &PlotData, id_filter: &[u16]) {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();
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
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();
    let plot_points = plot_vals.raw_plot_points();
    let filtered_points = filter_plot_points(plot_points, x_min_max_ext);

    let line = Line::new(filtered_points)
        .width(line_width)
        .name(plot_vals.label())
        .color(plot_vals.get_color())
        .highlight(plot_vals.get_highlight());
    plot_ui.line(line);
}

pub fn plot_raw_mqtt<'p>(
    plot_ui: &mut egui_plot::PlotUi<'p>,
    label: &str,
    plot_points: &'p [PlotPoint],
    line_width: f32,
    x_min_max_ext: (f64, f64),
) {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();

    let filtered_points = filter_plot_points(plot_points, x_min_max_ext);

    let line = Line::new(filtered_points).width(line_width).name(label);
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

/// Filter plot points based on the x plot bounds.
pub fn filter_plot_points(points: &[PlotPoint], x_range: (f64, f64)) -> PlotPoints<'_> {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();

    // Don't bother filtering if there's less than 1024 points
    if points.len() < 1024 {
        return PlotPoints::Borrowed(points);
    }

    let start_idx = points.partition_point(|point| point.x < x_range.0);
    let end_idx = points.partition_point(|point| point.x < x_range.1);

    // This is the case if we scroll such that none of the plot points are within the plot bounds
    // in that case we just plot a single point to avoid hiding the plot from the legend
    // which also shuffles the coloring of every line
    if start_idx == end_idx {
        // If we scrolled far to the right
        if start_idx == points.len() {
            PlotPoints::Borrowed(&points[start_idx - 2..=start_idx - 1])
        }
        // If we scrolled far to the left
        else {
            PlotPoints::Borrowed(&points[start_idx..=start_idx])
        }
    } else {
        PlotPoints::Borrowed(&points[start_idx..end_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_less_than_1024_points_no_filtering() {
        let points: Vec<PlotPoint> = (0..500)
            .map(|i| [i as f64, i as f64 + 1.0].into())
            .collect();
        let x_range = (100.0, 300.0);

        // Since points are less than 1024, no filtering should be done
        let result = filter_plot_points(&points, x_range);

        // Result should be identical to input
        assert_eq!(result.points(), &points);
    }

    #[test]
    fn test_more_than_1024_points_with_filtering() {
        let points: Vec<PlotPoint> = (0..1500)
            .map(|i| [i as f64, i as f64 + 1.0].into())
            .collect();
        let x_range = (100.0, 500.0);

        // Since the points are more than 1024, filtering should happen
        let result = filter_plot_points(&points, x_range);

        assert_eq!(result.points(), &points[100..500]);
    }

    #[test]
    fn test_range_outside_bounds_with_large_data() {
        let points: Vec<PlotPoint> = (0..1500)
            .map(|i| [i as f64, i as f64 + 1.0].into())
            .collect();
        let x_range = (2000.0, 3000.0);

        // Since range is outside the data points, we should get first and last points
        let result = filter_plot_points(&points, x_range);

        assert_eq!(result.points(), &[]);
    }
}
