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
