/// Adapted from: https://github.com/nchechulin/mipmap-1d
use num_traits::{FromPrimitive, Num, ToPrimitive};
use serde::{Deserialize, Serialize};

/// Creates several downsampled versions of given vector.
/// This data structure takes 2x space of original data.
/// Example:
/// ```rust
/// use mipmap_1d::MipMap1D;
///
/// let data = vec![2, 4, 6, 8, 9];
/// let mipmap = MipMap1D::new(data);
/// assert_eq!(mipmap.num_levels(), 4);
/// assert_eq!(*mipmap.get_level(0).unwrap(), [2, 4, 6, 8, 9]);
/// assert_eq!(*mipmap.get_level(1).unwrap(), [3, 7, 9]);
/// assert_eq!(*mipmap.get_level(2).unwrap(), [5, 9]);
/// assert_eq!(*mipmap.get_level(3).unwrap(), [7]);
/// assert_eq!(mipmap.get_level(4), None);
/// ```
#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

    /// Downsamples a vector to `ceil(len / 2)`` elements.
    /// Currently, downsampling is done by averaging the pair of elements
    fn downsample(source: &[T]) -> Vec<T> {
        source
            .chunks(2)
            .map(|pair| match pair.len() {
                1 => pair[0],
                2 => T::from_f64((pair[0] + pair[1]).to_f64().unwrap() / 2.0).unwrap(),
                _ => panic!("Unsound condition"),
            })
            .collect()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MipMap2D<T: Num + ToPrimitive + FromPrimitive> {
    data: Vec<Vec<Vec<(T, u64)>>>,
}

impl<T: Num + ToPrimitive + FromPrimitive + Copy> MipMap2D<T> {
    pub fn new(source: Vec<Vec<(T, u64)>>) -> Self {
        let mut data = vec![source.clone()];
        let mut current = source;

        while current.len() > 1 && current[0].len() > 1 {
            let mipmap = Self::downsample(&current);
            current = mipmap.clone();
            data.push(mipmap);
        }

        Self { data }
    }

    /// Returns the total number of downsampled levels.
    pub fn num_levels(&self) -> usize {
        self.data.len()
    }

    /// Returns the data on given level.
    /// Level `0` returns the source data; the higher the level, the higher the compression.
    /// If the level is out of bounds, returns None
    pub fn get_level(&self, level: usize) -> Option<&Vec<Vec<(T, u64)>>> {
        if level >= self.num_levels() {
            return None;
        }
        Some(&self.data[level])
    }

    /// Downsamples a 2D vector to approximately quarter the number of elements.
    /// Downsampling is done by averaging the values in a 2x2 grid and taking the average time.
    fn downsample(source: &[Vec<(T, u64)>]) -> Vec<Vec<(T, u64)>> {
        let rows = (source.len() + 1) / 2;
        let cols = (source[0].len() + 1) / 2;

        let mut result = vec![vec![(T::zero(), 0); cols]; rows];

        for i in 0..rows {
            for j in 0..cols {
                let mut sum = T::zero();
                let mut count = 0;
                let mut time_sum: u64 = 0;

                for di in 0..2 {
                    for dj in 0..2 {
                        if let Some(row) = source.get(i * 2 + di) {
                            if let Some(&(value, time)) = row.get(j * 2 + dj) {
                                sum = sum + value;
                                time_sum += time;
                                count += 1;
                            }
                        }
                    }
                }

                let avg_value = T::from_f64(sum.to_f64().unwrap() / count as f64).unwrap();
                let avg_time = time_sum / count as u64;
                result[i][j] = (avg_value, avg_time);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_2d() {
        let data = vec![
            vec![(2, 0), (4, 10), (6, 20), (8, 30)],
            vec![(3, 5), (5, 15), (7, 25), (9, 35)],
            vec![(4, 10), (6, 20), (8, 30), (10, 40)],
        ];

        let mipmap = MipMap2D::new(data);

        println!("Number of levels: {}", mipmap.num_levels());

        for level in 0..mipmap.num_levels() {
            println!("Level {}: {:?}", level, mipmap.get_level(level));
        }
    }
}
