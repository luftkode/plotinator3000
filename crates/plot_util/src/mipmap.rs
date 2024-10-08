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
    pub fn get_level(&self, level: usize) -> Option<&Vec<[T; 2]>> {
        if level >= self.num_levels() {
            return None;
        }

        Some(&self.data[level])
    }

    pub fn get_level_match(&self, pixel_width: usize, x_bounds: (usize, usize)) -> usize {
        let (x_min, x_max) = x_bounds;
        for (reversed_idx, lvl) in self.data.iter().rev().enumerate() {
            let real_idx = self.num_levels() - reversed_idx;

            // Find the index where the points fit within the minimum bounds and where it fits within the maximum
            // basically performs a binary search
            let start_idx =
                lvl.partition_point(|&x| x[0].to_usize().expect("Doesn't fit in usize") <= x_min);
            let end_idx =
                lvl.partition_point(|&x| x[0].to_usize().expect("Doesn't fit in usize") <= x_max);
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
                let index_bool: usize = (pairs[0][1] < pairs[1][1]) as usize;
                pairs[index_bool]
            },
            MipMapStrategy::Max => |pairs: &[[T; 2]]| {
                // Branchless way of selecting the point with the greatest X-value
                let index_bool: usize = (pairs[0][1] > pairs[1][1]) as usize;
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
