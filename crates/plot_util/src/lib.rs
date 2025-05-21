pub mod mipmap;

use std::ops::RangeInclusive;

use egui_plot::{Line, PlotPoint, PlotPoints};

pub mod plots;

pub use plots::{
    Plots,
    plot_data::{PlotData, PlotValues, StoredPlotLabels},
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
    let x_plot_bounds = plot_ui.plot_bounds().range_x();
    for plot_vals in plots {
        match mipmap_cfg {
            MipMapConfiguration::Disabled => {
                plot_raw(plot_ui, plot_vals, line_width, x_plot_bounds.clone());
            }
            MipMapConfiguration::Auto => {
                let (level, idx_range) =
                    plot_vals.get_scaled_mipmap_levels(plots_width_pixels, x_plot_bounds.clone());

                plot_with_mipmapping(
                    plot_ui,
                    plot_vals,
                    line_width,
                    level,
                    x_plot_bounds.clone(),
                    idx_range,
                );
            }
            MipMapConfiguration::Manual(level) => {
                plot_with_mipmapping(
                    plot_ui,
                    plot_vals,
                    line_width,
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
    line_width: f32,
    mipmap_lvl: usize,
    x_bounds: RangeInclusive<f64>,
    // if the range is already known then we can skip filtering
    known_idx_range: Option<(usize, usize)>,
) {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();

    let plot_points_minmax = plot_vals.get_level_or_max(mipmap_lvl);
    if plot_points_minmax.is_empty() {
        // In this case there was so few samples that downsampling just once was below the minimum threshold, so we just plot all samples
        plot_raw(plot_ui, plot_vals, line_width, x_bounds);
    } else {
        let plot_points_minmax = match known_idx_range {
            Some((start, end)) => PlotPoints::Borrowed(&plot_points_minmax[start..end]),
            None => filter_plot_points(plot_points_minmax, x_bounds),
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
    x_bounds: RangeInclusive<f64>,
) {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();
    let plot_points = plot_vals.raw_plot_points();
    let filtered_points = filter_plot_points(plot_points, x_bounds);

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
    x_bounds: RangeInclusive<f64>,
) {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();

    let filtered_points = filter_plot_points(plot_points, x_bounds);

    let line = Line::new(filtered_points).width(line_width).name(label);
    plot_ui.line(line);
}

/// Filter plot points based on the x plot bounds.
pub fn filter_plot_points(points: &[PlotPoint], x_bounds: RangeInclusive<f64>) -> PlotPoints<'_> {
    #[cfg(all(feature = "profiling", not(target_arch = "wasm32")))]
    puffin::profile_function!();

    // Don't bother filtering if there's less than 1024 points
    if points.len() < 1024 {
        return PlotPoints::Borrowed(points);
    }

    let start_idx = points.partition_point(|point| point.x < *x_bounds.start());
    let end_idx = points.partition_point(|point| point.x < *x_bounds.end());

    let range: usize = end_idx - start_idx;

    // The range is 0 if we scroll such that none OR one of the plot points are within the plot bounds
    // in that case we plot the closest two points on either side of plot bounds.
    let (start, end) = if range == 0 {
        // No points in range - find closest points on either side
        // 3 cases to cover: (and yes they all happen in practice)
        // 1. Start index equals 0: add 2 to end index
        // 2. End index equals slice length: subtract 2 from start index
        // 3. The rest: subtract 1 from start index and add 1 to end index
        match (start_idx, end_idx) {
            (0, _) => (0, end_idx + 2),
            (_, end) if end == points.len() => (start_idx.saturating_sub(2), end),
            _ => (start_idx - 1, end_idx + 1),
        }
    } else {
        // Some points in range - add one point on each side when possible
        (start_idx.saturating_sub(1), (end_idx + 1).min(points.len()))
    };

    let filtered_points = PlotPoints::Borrowed(&points[start..end]);

    debug_assert!(
        filtered_points.points().len() >= 2,
        "Filtered points should always return at least 2 points!"
    );
    filtered_points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_less_than_1024_points_no_filtering() {
        let points: Vec<PlotPoint> = (0..500)
            .map(|i| [i as f64, i as f64 + 1.0].into())
            .collect();
        let x_range = 100.0..=300.0;

        // Since points are less than 1024, no filtering should be done
        let result = filter_plot_points(&points, x_range);

        // Result should be identical to input
        assert_eq!(result.points(), &points);
    }

    #[test]
    fn test_more_than_1024_points_with_filtering() {
        let points: Vec<PlotPoint> = (0..1500)
            .map(|i| [i as f64, i as f64 + 0.2].into())
            .collect();
        let (x_min, x_max) = (100.1, 500.1); // .1 to avoid bounds and plot bounds that are "exactly equal" (as f64 is flaky with that)
        let expected_x_min = x_min as usize; // Shaves off decimal so it's like subtracting 1
        let expected_x_max = x_max as usize + 1;

        // Since the points are more than 1024, filtering should happen
        let result = filter_plot_points(&points, x_min..=x_max);

        assert_eq!(*result.points().first().unwrap(), points[expected_x_min]);
        assert_eq!(*result.points().last().unwrap(), points[expected_x_max]);
        pretty_assertions::assert_eq!(result.points(), &points[expected_x_min..=expected_x_max]);
    }

    #[test]
    fn test_range_outside_bounds_to_the_right_with_large_data() {
        let points: Vec<PlotPoint> = (0..1500)
            .map(|i| [i as f64, i as f64 + 1.0].into())
            .collect();
        let x_range = 2000.0..=3000.0;

        // Since range is outside the data points we expect to get the two closest points to the bounds
        let expected_result = &points[1498..=1499];

        let result = filter_plot_points(&points, x_range);

        assert_eq!(result.points(), expected_result);
    }

    #[test]
    fn test_range_outside_bounds_to_the_left_with_large_data() {
        let points: Vec<PlotPoint> = (1500..3000)
            .map(|i| [i as f64, i as f64 + 1.0].into())
            .collect();
        let x_range = 0.0..=100.0;

        // Since range is outside the data points we expect to just get the first two points
        let expected_result = &points[0..=1];

        let result = filter_plot_points(&points, x_range);

        assert_eq!(result.points(), expected_result);
    }
}
