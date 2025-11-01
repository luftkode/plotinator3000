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
        let mipmap_max_pp = MipMap2DPlotPoints::without_base(
            points,
            MipMapStrategy::Max,
            Self::MIPMAP_MIN_ELEMENTS,
        );
        let mut mipmap_min_pp = MipMap2DPlotPoints::without_base(
            points,
            MipMapStrategy::Min,
            Self::MIPMAP_MIN_ELEMENTS,
        );
        mipmap_min_pp.join(&mipmap_max_pp);
        mipmap_min_pp
    }

    fn new(source: &[[f64; 2]], strategy: MipMapStrategy, min_elements: usize) -> Self {
        let base: Vec<PlotPoint> = source.iter().map(|&p| PlotPoint::from(p)).collect();
        let mut data: Vec<Vec<PlotPoint>> =
            Vec::with_capacity(estimate_levels(base.len(), min_elements));
        data.push(base);

        while data.last().expect("unsound condition").len() > min_elements {
            let next = Self::downsample(data.last().expect("unsound condition"), strategy);
            data.push(next);
        }

        Self {
            data,
            most_recent_lookup: Cell::new(LevelLookupCached::default()),
        }
    }

    /// Create a [`MipMap2DPlotPoints`] but don't include the base level. Retrieving level 0 will then return an empty vec.
    ///
    /// Useful to avoid multiple redundant copies of the source if creating multiple [`MipMap2DPlotPoints`] from the same source.
    pub fn without_base(
        source: &[[f64; 2]],
        strategy: MipMapStrategy,
        min_elements: usize,
    ) -> Self {
        // Reserve +1 for the empty base placeholder
        let mut data: Vec<Vec<PlotPoint>> =
            Vec::with_capacity(estimate_levels(source.len(), min_elements) + 1);
        data.push(Vec::new()); // level 0 is empty by design

        // First real level
        let first: Vec<PlotPoint> = source.iter().map(|&p| PlotPoint::from(p)).collect();
        let mut lvl = Self::downsample(&first, strategy);
        data.push(lvl);

        while data.last().expect("unsound condition").len() > min_elements {
            lvl = Self::downsample(data.last().expect("unsound condition"), strategy);
            data.push(lvl);
        }

        Self {
            data,
            most_recent_lookup: Cell::new(LevelLookupCached::default()),
        }
    }

    /// Downsample a vector to `ceil(len / 2)` elements with the chosen [`MipMapStrategy`]
    fn downsample(source: &[PlotPoint], strategy: MipMapStrategy) -> Vec<PlotPoint> {
        let pairs = source.len() / 2;
        let rem = source.len() % 2;

        let mut out = Vec::with_capacity(pairs + rem);
        for pair in source.chunks_exact(2) {
            out.push(Self::pick(pair[0], pair[1], strategy));
        }
        if rem == 1 {
            out.push(*source.last().expect("unsound condition"));
        }
        out
    }

    #[inline(always)]
    fn pick(p0: PlotPoint, p1: PlotPoint, strategy: MipMapStrategy) -> PlotPoint {
        // branchless, don't touch unless you understand it
        match strategy {
            MipMapStrategy::Min => {
                // choose lower y
                let idx: usize = (p0.y > p1.y) as usize;
                [p0, p1][idx]
            }
            MipMapStrategy::Max => {
                // choose higher y
                let idx: usize = (p0.y < p1.y) as usize;
                [p0, p1][idx]
            }
        }
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
    fn test_mipmap_strategy_max() {
        let source: Vec<[f64; 2]> = vec![[1.1, 2.2], [3.3, 4.4], [5.5, 1.1], [7.7, 3.3]];
        let mipmap = MipMap2DPlotPoints::new(&source, MipMapStrategy::Max, 1);

        // Level 1 should keep the points with higher y-values from each pair
        let expected_level_1: Vec<PlotPoint> = vec![source[1].into(), source[3].into()];
        assert_eq!(mipmap.get_level(1), Some(expected_level_1.as_slice()));

        // Level 2 should keep the point with the higher y-value from level 1
        let expected_level_2 = vec![PlotPoint::new(3.3, 4.4)];
        assert_eq!(mipmap.get_level(2), Some(expected_level_2.as_slice()));
    }

    #[test]
    fn test_mipmap_strategy_min() {
        let source: Vec<[f64; 2]> = vec![[1.1, 2.2], [3.3, 4.4], [5.5, 1.1], [7.7, 3.3]];
        let mipmap = MipMap2DPlotPoints::new(&source, MipMapStrategy::Min, 1);

        // Level 1 should keep the points with lower y-values from each pair
        let expected_level_1: Vec<PlotPoint> = vec![source[0].into(), source[2].into()];
        assert_eq!(mipmap.get_level(1), Some(expected_level_1.as_slice()));

        // Level 2 should keep the point with the lower y-value from level 1
        let expected_level_2: Vec<_> = vec![source[2].into()];
        assert_eq!(mipmap.get_level(2), Some(expected_level_2.as_slice()));
    }

    #[test]
    fn test_different_strategies() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 8.0], [5.0, 4.0], [7.0, 10.0]];

        let min_mipmap = MipMap2DPlotPoints::new(&source, MipMapStrategy::Min, 1);
        let max_mipmap = MipMap2DPlotPoints::new(&source, MipMapStrategy::Max, 1);

        // Test level 1 for each strategy
        let expected_min: Vec<PlotPoint> = vec![source[0].into(), source[2].into()];
        let expected_max: Vec<PlotPoint> = vec![source[1].into(), source[3].into()];

        assert_eq!(min_mipmap.get_level(1), Some(expected_min.as_slice()));
        assert_eq!(max_mipmap.get_level(1), Some(expected_max.as_slice()));
    }

    #[test]
    fn test_level_match() {
        // Length of 16 will yield 5 levels of length:
        // - [0]: 16
        // - [1]: 8
        // - [2]: 4
        // - [3]: 2
        // - [4]: 1
        let source_len = 16;
        let source: Vec<[f64; 2]> = (0..source_len).map(|i| [i as f64, i as f64]).collect();
        let mipmap = MipMap2DPlotPoints::new(&source, MipMapStrategy::Min, 1);

        for (pixel_width, expected_lvl, expected_range) in [
            (1usize, 3usize, Some((0, 2))),
            (2, 2, Some((0, 4))),
            (4, 1, Some((0, 8))),
            (8, 0, Some((0, 16))),
            (16, 0, None),
        ] {
            let (lvl, range_res) = mipmap.get_level_match(pixel_width, 0.0..=15.);
            assert_eq!(
                lvl, expected_lvl,
                "Expected lvl {expected_lvl} for width: {pixel_width}"
            );
            assert_eq!(
                range_res, expected_range,
                "Expected range {expected_range:?} for width: {pixel_width}"
            );
        }
    }

    /// Test for: `<https://github.com/luftkode/plotinator3000/issues/62>`
    #[test]
    fn test_level_match_large_timestamps() {
        let source_len = 1600;
        let source: Vec<[f64; 2]> = (0..source_len)
            .map(|i| [i as f64 + UNIX_TS_NS, i as f64 + UNIX_TS_NS])
            .collect();
        let mipmap = MipMap2DPlotPoints::new(&source, MipMapStrategy::Min, 1);

        let x_bounds = UNIX_TS_NS + 300.0..=UNIX_TS_NS + 1500.;

        for (pixel_width, expected_lvl, expected_range) in [
            (100usize, 3usize, Some((16, 177))),
            (200, 2, Some((32, 353))),
            (400, 1, Some((64, 705))),
            (800, 0, Some((128, 1409))),
            (1600, 0, None),
        ] {
            let (lvl, range_res) = mipmap.get_level_match(pixel_width, x_bounds.clone());
            assert_eq!(
                lvl, expected_lvl,
                "Expected lvl {expected_lvl} for width: {pixel_width}"
            );

            assert_eq!(
                range_res, expected_range,
                "Expected range {expected_range:?} for width: {pixel_width}"
            );
        }
    }

    #[test]
    fn test_out_of_bounds_level() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 4.0]];
        let mipmap = MipMap2DPlotPoints::new(&source, MipMapStrategy::Min, 1);

        assert_eq!(mipmap.get_level(2), None);
    }

    #[test]
    fn test_single_element_source() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0]];
        let mipmap = MipMap2DPlotPoints::new(&source, MipMapStrategy::Max, 1);

        assert_eq!(mipmap.num_levels(), 1);
        assert_eq!(
            mipmap.get_level(0),
            Some(vec![PlotPoint::from(source[0])].as_slice())
        );
    }
}
