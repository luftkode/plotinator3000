/// Adapted from: <https://github.com/nchechulin/mipmap-1d/>
use num_traits::{FromPrimitive, Num, ToPrimitive};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MipMap1D<T: Num + ToPrimitive + FromPrimitive> {
    data: Vec<Vec<T>>,
}

impl<T: Num + ToPrimitive + FromPrimitive + Copy> MipMap1D<T> {
    pub fn new(source: Vec<T>) -> Self {
        let mut data = vec![source.clone()];
        let mut current = source;

        while current.len() > 1 {
            let mipmap = Self::downsample(&current);
            current.clone_from(&mipmap);
            data.push(mipmap);
        }

        Self { data }
    }

    /// Returns the total number of downsampled levels.
    /// Equal to `ceil(log2(source.len())`
    pub fn num_levels(&self) -> usize {
        self.data.len()
    }

    /// Returns the data on given level.
    /// Level `0` returns the source data; the higher the level, the higher the compression (i.e. smaller vectors are returned).
    /// If the level is out of bounds, returns None
    pub fn get_level(&self, level: usize) -> Option<&Vec<T>> {
        if level >= self.num_levels() {
            return None;
        }

        Some(&self.data[level])
    }

    /// Downsamples a vector to `ceil(len / 2)` elements.
    /// Currently, downsampling is done by averaging the pair of elements
    fn downsample(source: &[T]) -> Vec<T> {
        source
            .chunks(2)
            .map(|pair| match pair.len() {
                1 => pair[0],
                2 => T::from_f64(
                    (pair[0] + pair[1])
                        .to_f64()
                        .expect("Value not representable as an f64")
                        / 2.0,
                )
                .expect("Value not representable as an f64"),
                _ => panic!("Unsound condition"),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MipMapStrategy {
    Linear,
    Min,
    Max,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MipMap2D<T: Num + ToPrimitive + FromPrimitive + PartialOrd> {
    strategy: MipMapStrategy,
    data: Vec<Vec<[T; 2]>>,
}

impl<T: Num + ToPrimitive + FromPrimitive + Copy + PartialOrd> MipMap2D<T> {
    pub fn new(source: Vec<[T; 2]>, strategy: MipMapStrategy) -> Self {
        let mut data = vec![source.clone()];
        let mut current = source;

        while current.len() > 1 {
            let mipmap = Self::downsample(&current, strategy);
            current.clone_from(&mipmap);
            data.push(mipmap);
        }

        Self { data, strategy }
    }

    /// Returns the total number of downsampled levels.
    /// Equal to `ceil(log2(source.len())`
    pub fn num_levels(&self) -> usize {
        self.data.len()
    }

    /// Returns the data on given level.
    /// Level `0` returns the source data; the higher the level, the higher the compression (i.e. smaller vectors are returned).
    /// If the level is out of bounds, returns None
    pub fn get_level(&self, level: usize) -> Option<&[[T; 2]]> {
        if level >= self.num_levels() {
            return None;
        }

        Some(self.data[level].as_slice())
    }

    /// Convenience function to get a level or return the highest if the requested level is higher or equal to the max
    pub fn get_level_or_max(&self, level: usize) -> &[[T; 2]] {
        if level >= self.num_levels() {
            return self.get_max_level();
        }

        &self.data[level]
    }

    /// Get the highest level of downsampling
    pub fn get_max_level(&self) -> &[[T; 2]] {
        &self.data[self.num_levels() - 1]
    }

    pub fn get_level_match(&self, pixel_width: usize, x_bounds: (usize, usize)) -> usize {
        let (x_min, x_max) = x_bounds;
        for (reversed_idx, lvl) in self.data.iter().rev().enumerate() {
            let real_idx = self.num_levels() - reversed_idx;

            // Find the index where the points fit within the minimum bounds and where it fits within the maximum
            // basically performs a binary search
            let start_idx =
                lvl.partition_point(|&x| x[0].to_usize().expect("Doesn't fit in usize") < x_min);
            let end_idx =
                lvl.partition_point(|&x| x[0].to_usize().expect("Doesn't fit in usize") < x_max);
            // Calculate the count of points within bounds
            let count_within_bounds = end_idx.saturating_sub(start_idx);

            if count_within_bounds > pixel_width * 2 {
                return real_idx;
            }
        }
        // If we didn't find an appropriate scaled level, we return the max resolution
        0
    }

    /// Downsamples a vector to `ceil(len / 2)` elements.
    /// Currently, downsampling is done by averaging the pair of elements
    fn downsample(source: &[[T; 2]], strategy: MipMapStrategy) -> Vec<[T; 2]> {
        let strategy = match strategy {
            MipMapStrategy::Linear => |point_pair: &[[T; 2]]| {
                let x = Self::linear_interpolate(point_pair[0][0], point_pair[1][0])
                    .expect("Interpolation error, this is a bug, please report it");
                let y = Self::linear_interpolate(point_pair[0][1], point_pair[1][1])
                    .expect("Interpolation error, this is a bug, please report it");
                [x, y]
            },
            MipMapStrategy::Min => |pairs: &[[T; 2]]| {
                // Branchless way of selecting the point with the smallest X-value
                let index_bool: usize = (pairs[0][1] > pairs[1][1]) as usize;
                pairs[index_bool]
            },
            MipMapStrategy::Max => |pairs: &[[T; 2]]| {
                // Branchless way of selecting the point with the greatest X-value
                let index_bool: usize = (pairs[0][1] < pairs[1][1]) as usize;
                pairs[index_bool]
            },
        };
        source
            .chunks(2)
            .map(|pairs| match pairs.len() {
                1 => pairs[0],
                2 => strategy(pairs),
                _ => unreachable!("Unsound condition"),
            })
            .collect()
    }

    fn linear_interpolate(a: T, b: T) -> Option<T> {
        let res = T::from_f64((a + b).to_f64()? / 2.0)?;
        Some(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mipmap_strategy_max() {
        let source: Vec<[f64; 2]> = vec![[1.1, 2.2], [3.3, 4.4], [5.5, 1.1], [7.7, 3.3]];
        let mipmap = MipMap2D::new(source, MipMapStrategy::Max);

        // Level 1 should keep the points with higher y-values from each pair
        let expected_level_1: Vec<[f64; 2]> = vec![[3.3, 4.4], [7.7, 3.3]];
        assert_eq!(mipmap.get_level(1), Some(expected_level_1.as_slice()));

        // Level 2 should keep the point with the higher y-value from level 1
        let expected_level_2: Vec<[f64; 2]> = vec![[3.3, 4.4]];
        assert_eq!(mipmap.get_level(2), Some(expected_level_2.as_slice()));
    }

    #[test]
    fn test_mipmap_strategy_min() {
        let source: Vec<[f64; 2]> = vec![[1.1, 2.2], [3.3, 4.4], [5.5, 1.1], [7.7, 3.3]];
        let mipmap = MipMap2D::new(source, MipMapStrategy::Min);

        // Level 1 should keep the points with lower y-values from each pair
        let expected_level_1: Vec<[f64; 2]> = vec![[1.1, 2.2], [5.5, 1.1]];
        assert_eq!(mipmap.get_level(1), Some(expected_level_1.as_slice()));

        // Level 2 should keep the point with the lower y-value from level 1
        let expected_level_2: Vec<[f64; 2]> = vec![[5.5, 1.1]];
        assert_eq!(mipmap.get_level(2), Some(expected_level_2.as_slice()));
    }

    #[test]
    fn test_mipmap_strategy_linear() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [7.0, 8.0]];
        let mipmap = MipMap2D::new(source.clone(), MipMapStrategy::Linear);

        assert_eq!(mipmap.num_levels(), 3);
        assert_eq!(mipmap.get_level(0), Some(source.as_slice()));

        let expected_level_1: Vec<[f64; 2]> = vec![[2.0, 3.0], [6.0, 7.0]];
        assert_eq!(mipmap.get_level(1), Some(expected_level_1.as_slice()));

        let expected_level_2: Vec<[f64; 2]> = vec![[4.0, 5.0]];
        assert_eq!(mipmap.get_level(2), Some(expected_level_2.as_slice()));
    }

    #[test]
    fn test_different_strategies() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 8.0], [5.0, 4.0], [7.0, 10.0]];

        let linear_mipmap = MipMap2D::new(source.clone(), MipMapStrategy::Linear);
        let min_mipmap = MipMap2D::new(source.clone(), MipMapStrategy::Min);
        let max_mipmap = MipMap2D::new(source.clone(), MipMapStrategy::Max);

        // Test level 1 for each strategy
        let expected_linear: Vec<[f64; 2]> = vec![[2.0, 5.0], [6.0, 7.0]];
        let expected_min: Vec<[f64; 2]> = vec![[1.0, 2.0], [5.0, 4.0]];
        let expected_max: Vec<[f64; 2]> = vec![[3.0, 8.0], [7.0, 10.0]];

        assert_eq!(linear_mipmap.get_level(1), Some(expected_linear.as_slice()));
        assert_eq!(min_mipmap.get_level(1), Some(expected_min.as_slice()));
        assert_eq!(max_mipmap.get_level(1), Some(expected_max.as_slice()));
    }

    #[test]
    fn test_get_level_or_max() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 4.0]];
        let mipmap = MipMap2D::new(source.clone(), MipMapStrategy::Linear);

        assert_eq!(mipmap.get_level_or_max(0), &[[1.0, 2.0], [3.0, 4.0]]);
        assert_eq!(mipmap.get_level_or_max(1), &[[2.0, 3.0]]);
        assert_eq!(mipmap.get_level_or_max(2), &[[2.0, 3.0]]); // Returns max level
    }

    #[test]
    fn test_get_max_level() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [7.0, 8.0]];
        let mipmap = MipMap2D::new(source, MipMapStrategy::Linear);

        assert_eq!(mipmap.get_max_level(), &[[4.0, 5.0]]);
    }

    #[test]
    fn test_level_match() {
        let source: Vec<[f64; 2]> = (0..16).map(|i| [i as f64, i as f64]).collect();
        let mipmap = MipMap2D::new(source, MipMapStrategy::Min);

        assert_eq!(mipmap.get_level_match(1, (0, 15)), 3);
        assert_eq!(mipmap.get_level_match(2, (0, 15)), 2);
        assert_eq!(mipmap.get_level_match(4, (0, 15)), 1);
        assert_eq!(mipmap.get_level_match(8, (0, 15)), 0);
        assert_eq!(mipmap.get_level_match(16, (0, 15)), 0);
    }

    #[test]
    fn test_out_of_bounds_level() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0], [3.0, 4.0]];
        let mipmap = MipMap2D::new(source, MipMapStrategy::Min);

        assert_eq!(mipmap.get_level(2), None);
    }

    #[test]
    fn test_single_element_source() {
        let source: Vec<[f64; 2]> = vec![[1.0, 2.0]];
        let mipmap = MipMap2D::new(source.clone(), MipMapStrategy::Max);

        assert_eq!(mipmap.num_levels(), 1);
        assert_eq!(mipmap.get_level(0), Some(source.as_slice()));
    }
}
