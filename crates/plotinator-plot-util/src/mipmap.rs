use egui_plot::PlotPoint;
use serde::{Deserialize, Serialize};
use std::{cell::Cell, ops::RangeInclusive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MipMapStrategy {
    Min,
    Max,
}

#[inline]
fn estimate_levels(mut len: usize, min_elements: usize) -> usize {
    if len == 0 {
        return 1;
    }
    let mut levels = 1; // include base
    while len > min_elements {
        len = len.div_ceil(2);
        levels += 1;
    }
    levels
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct LevelLookupCached {
    pixel_width: usize,
    x_bounds: (f64, f64),
    result_span: Option<(usize, usize)>,
    result_idx: usize,
}

impl LevelLookupCached {
    #[inline]
    pub fn is_equal(&self, pixel_width: usize, x_bounds: (f64, f64)) -> bool {
        // Compare bounds first because pixel_width is much more stable
        self.x_bounds == x_bounds && self.pixel_width == pixel_width
    }
}

pub struct MipMap2DPlotPoints {
    data: Vec<Vec<PlotPoint>>,
    most_recent_lookup: Cell<LevelLookupCached>,
}

impl MipMap2DPlotPoints {
    // Don't mipmap/downsample to more than this amount of elements
    const MIPMAP_MIN_ELEMENTS: usize = 512;

    pub fn minmax(points: &[[f64; 2]]) -> Self {
        let base: Vec<PlotPoint> = points.iter().map(|&p| PlotPoint::from(p)).collect();
        let mut data: Vec<Vec<PlotPoint>> =
            Vec::with_capacity(estimate_levels(base.len(), Self::MIPMAP_MIN_ELEMENTS));

        data.push(base);

        while data.last().expect("unsound condition").len() > Self::MIPMAP_MIN_ELEMENTS {
            let next = Self::downsample_minmax(data.last().expect("unsound condition"));
            data.push(next);
        }

        Self {
            data,
            most_recent_lookup: Cell::new(LevelLookupCached::default()),
        }
    }

    #[inline(always)]
    fn downsample_minmax(source: &[PlotPoint]) -> Vec<PlotPoint> {
        let chunks = source.len() / 4;
        let rem = source.len() % 4;

        let mut out = Vec::with_capacity(chunks * 2 + rem);

        for chunk in source.chunks_exact(4) {
            let mut min_point = chunk[0];
            let mut max_point = chunk[0];

            for &point in &chunk[1..] {
                if point.y < min_point.y {
                    min_point = point;
                }
                if point.y > max_point.y {
                    max_point = point;
                }
            }

            if min_point.x < max_point.x {
                out.push(min_point);
                out.push(max_point);
            } else {
                out.push(max_point);
                out.push(min_point);
            }
        }

        // Handle remainder
        if rem > 0 {
            let remainder = &source[chunks * 4..];
            out.extend_from_slice(remainder);
        }

        out
    }

    /// Returns the total number of downsampled levels.
    /// Equal to `ceil(log2(source.len())`
    pub fn num_levels(&self) -> usize {
        self.data.len()
    }

    /// Returns the data on given level.
    /// Level `0` returns the source data; the higher the level, the higher the compression (i.e. smaller vectors are returned).
    /// If the level is out of bounds, returns None
    pub fn get_level(&self, level: usize) -> Option<&[PlotPoint]> {
        if level >= self.num_levels() {
            return None;
        }

        Some(self.data[level].as_slice())
    }

    /// Convenience function to get a level or return the highest if the requested level is higher or equal to the max
    pub fn get_level_or_max(&self, level: usize) -> &[PlotPoint] {
        if level >= self.num_levels() {
            return self.get_max_level();
        }

        &self.data[level]
    }

    /// Get the highest level of downsampling
    pub fn get_max_level(&self) -> &[PlotPoint] {
        &self.data[self.num_levels() - 1]
    }

    /// Retrieves the index of the level that matches the specified pixel width
    /// and bounds within the dataset, using cached results if available.
    ///
    /// The bounds (`x_bounds`) are specified as a tuple `(x_min, x_max)`, which
    /// defines the range within which the data points must fall. The pixel width
    /// indicates how many data points should be considered in the calculation.
    ///
    /// # Parameters
    ///
    /// - `pixel_width`: The width in pixels that influences the number of points
    ///   considered for each level. This effectively determines the resolution of
    ///   the search.
    /// - `x_bounds`: A tuple containing two `usize` values `(x_min, x_max)` that
    ///   define the lower and upper bounds of the data points to be considered
    ///   for matching. These bounds are used to filter the relevant points in
    ///   each level.
    ///
    /// # Returns
    ///
    /// The level that matches the requirement (or the highest resolution), and if
    /// the match was found in a level below the max resolution, also returns a tuple
    /// of the start and end index of the level that matches the requirement.
    pub fn get_level_match(
        &self,
        pixel_width: usize,
        x_bounds: RangeInclusive<f64>,
    ) -> (usize, Option<(usize, usize)>) {
        let (x_min, x_max) = (*x_bounds.start(), *x_bounds.end());
        let cached = self.most_recent_lookup.get();
        if cached.is_equal(pixel_width, (x_min, x_max)) {
            return (cached.result_idx, cached.result_span);
        }
        let target_point_count = pixel_width;

        // Avoid repeated calls to num_levels()
        let num_levels = self.num_levels();

        // If not found in cache, compute it
        for lvl_idx in (0..num_levels).rev() {
            let lvl = &self.data[lvl_idx];

            // Skip if the level doesn't have enough points even without accounting for plot bounds
            if lvl.len() <= target_point_count {
                continue;
            }

            let start_idx = lvl.partition_point(|p| p.x < x_min);
            let end_idx = lvl.partition_point(|p| p.x < x_max);

            // Use saturating_sub for safety and to avoid potential panic
            let count_within_bounds = end_idx.saturating_sub(start_idx);

            if count_within_bounds > target_point_count {
                // add one point on each side when possible, so that the lines
                // are drawn correctly to the next point outside of the screen
                let extended_start_idx = start_idx.saturating_sub(1);
                let extended_end_idx = (end_idx + 1).min(lvl.len());
                let new_cached = LevelLookupCached {
                    pixel_width,
                    x_bounds: (x_min, x_max),
                    result_span: Some((extended_start_idx, extended_end_idx)),
                    result_idx: lvl_idx,
                };
                self.most_recent_lookup.set(new_cached);
                return (lvl_idx, Some((extended_start_idx, extended_end_idx)));
            }
        }
        self.most_recent_lookup.set(LevelLookupCached {
            pixel_width,
            x_bounds: (x_min, x_max),
            result_span: None,
            result_idx: 0,
        });
        (0, None)
    }

    pub fn join(&mut self, other: &Self) {
        // For each level, combine, sort by timestamp, and deduplicate
        for level in 0..self.num_levels() {
            let mut merged = self.data[level].clone();
            merged.extend_from_slice(other.get_level_or_max(level));
            merged.sort_unstable_by(|a, b| a.x.partial_cmp(&b.x).expect("Invalid timestamp"));
            merged.dedup_by(|a, b| a.x == b.x); // Remove consecutive elements with same timestamp
            self.data[level] = merged;
        }

        // Reset the lookup cache since we've modified the data
        self.most_recent_lookup
            .replace(LevelLookupCached::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const UNIX_TS_NS: f64 = 1_728_470_564_000_000_000.0;

    #[test]
    fn test_minmax_basic() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 8.0], [5.0, 4.0], [7.0, 10.0]];
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        let lvl0 = mipmap.get_level(0).unwrap();
        assert_eq!(mipmap.num_levels(), 1);
        assert_eq!(lvl0.len(), 4);
    }

    #[test]
    fn test_minmax_contains_both_extremes() {
        let source: Vec<[f64; 2]> = vec![[1.0, 5.0], [2.0, 1.0], [3.0, 9.0], [4.0, 2.0]];
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        let lvl0 = mipmap.get_level(0).unwrap();
        assert_eq!(mipmap.num_levels(), 1);
        let y_values: Vec<f64> = lvl0.iter().map(|p| p.y).collect();

        insta::assert_debug_snapshot!(y_values);
    }

    #[test]
    fn test_minmax_level_count() {
        let source: Vec<[f64; 2]> = (0..8096).map(|i| [i as f64, i as f64]).collect();
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        assert_eq!(mipmap.num_levels(), 5);
        let mut level_sizes = vec![];
        for i in 0..mipmap.num_levels() {
            level_sizes.push(mipmap.get_level(i).unwrap().len());
        }

        let first_5_elements: Vec<_> = mipmap.get_max_level().iter().take(5).collect();

        insta::assert_debug_snapshot!((level_sizes, first_5_elements));
    }

    #[test]
    fn test_minmax_sorted_by_x() {
        let source: Vec<[f64; 2]> = vec![[1.0, 5.0], [2.0, 1.0], [3.0, 9.0], [4.0, 2.0]];
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        let level_1 = mipmap.get_level(0).unwrap();
        for i in 1..level_1.len() {
            assert!(level_1[i - 1].x <= level_1[i].x);
        }
    }

    #[test]
    fn test_level_match() {
        let source_len = 2048;
        let source: Vec<[f64; 2]> = (0..source_len).map(|i| [i as f64, i as f64]).collect();
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        // Test various pixel widths
        for pixel_width in [100, 200, 400, 800, 1600] {
            let (lvl, range_res) =
                mipmap.get_level_match(pixel_width, 0.0..=(source_len as f64 - 1.0));

            assert!(lvl < mipmap.num_levels());

            // If a range is returned, verify it's within bounds
            if let Some((start, end)) = range_res {
                let level_data = mipmap.get_level(lvl).unwrap();
                assert!(start < end);
                assert!(end <= level_data.len());
            }
        }
    }

    /// Test for: `<https://github.com/luftkode/plotinator3000/issues/62>`
    #[test]
    fn test_level_match_large_timestamps() {
        let source_len = 1600;
        let source: Vec<[f64; 2]> = (0..source_len)
            .map(|i| [i as f64 + UNIX_TS_NS, i as f64 + UNIX_TS_NS])
            .collect();
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        let x_bounds = UNIX_TS_NS + 300.0..=UNIX_TS_NS + 1500.0;

        for pixel_width in [100, 200, 400, 800, 1600] {
            let (lvl, range_res) = mipmap.get_level_match(pixel_width, x_bounds.clone());

            assert!(lvl < mipmap.num_levels());

            // If a range is returned, verify it's within bounds
            if let Some((start, end)) = range_res {
                let level_data = mipmap.get_level(lvl).unwrap();
                assert!(start < end);
                assert!(end <= level_data.len());
            }
        }
    }

    #[test]
    fn test_out_of_bounds_level() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 4.0]];
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        assert_eq!(mipmap.get_level(10), None);
    }

    #[test]
    fn test_single_element_source() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0]];
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        assert_eq!(mipmap.num_levels(), 1);
        assert_eq!(
            mipmap.get_level(0),
            Some(vec![PlotPoint::from(source[0])].as_slice())
        );
    }

    #[test]
    fn test_get_level_or_max() {
        let source: Vec<[f64; 2]> = (0..100).map(|i| [i as f64, i as f64]).collect();
        let mipmap = MipMap2DPlotPoints::minmax(&source);

        let max_level = mipmap.get_max_level();
        let result = mipmap.get_level_or_max(999);

        assert_eq!(result.len(), max_level.len());
        assert_eq!(result, max_level);
    }
}
