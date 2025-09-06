use std::ops::RangeInclusive;

use egui_plot::{PlotPoint, PlotPoints};

/// Filter plot points based on the x plot bounds.
pub fn filter_plot_points(points: &[PlotPoint], x_bounds: RangeInclusive<f64>) -> PlotPoints<'_> {
    plotinator_macros::profile_function!();

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
