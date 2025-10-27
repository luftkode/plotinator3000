use ndarray::{ArrayBase, Data, Ix1};

use crate::rawplot::TimeStampPrimitive;

/// Calculates distances between consecutive timestamps
///
/// Takes a collection of unix nanosecond timestamps and returns a vector
/// where each entry contains [timestamp, `distance_to_previous`].
/// The first entry has a distance of 0.0.
///
/// # Example
/// ```
/// # use plotinator_log_if::algorithms::timestamp_distances;
/// let timestamps = vec![1000i64, 1500, 2100];
/// let result = timestamp_distances(&timestamps);
/// assert_eq!(result, vec![[1000.0, 0.0], [1500.0, 500.0], [2100.0, 600.0]]);
/// ```
pub fn timestamp_distances(timestamps: &[impl TimeStampPrimitive]) -> Vec<[f64; 2]> {
    if timestamps.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(timestamps.len());

    // First timestamp has distance 0
    result.push([timestamps[0].as_(), 0.0]);

    // Use windows for efficient pairwise iteration
    for window in timestamps.windows(2) {
        let distance = (window[1] - window[0]).as_();
        result.push([window[1].as_(), distance]);
    }

    result
}

/// Calculates distances between consecutive timestamps from an ndarray
///
/// Takes an ndarray of unix nanosecond timestamps and returns a vector
/// where each entry contains [timestamp,  `distance_to_previous`].
///
/// # Example
/// ```
/// # use plotinator_log_if::algorithms::timestamp_distances_ndarray;
/// use ndarray::array;
///
/// let timestamps = array![1000i64, 1500, 2100];
/// let result = timestamp_distances_ndarray(&timestamps);
/// assert_eq!(result, vec![[1000.0, 0.0], [1500.0, 500.0], [2100.0, 600.0]]);
/// ```
pub fn timestamp_distances_ndarray<T, S>(timestamps: &ArrayBase<S, Ix1>) -> Vec<[f64; 2]>
where
    T: TimeStampPrimitive,
    S: Data<Elem = T>,
{
    if timestamps.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(timestamps.len());

    // First timestamp has distance 0
    result.push([timestamps[0].as_(), 0.0]);

    // Use windows for efficient pairwise iteration
    for window in timestamps.windows(2) {
        let distance = (window[1] - window[0]).as_();
        result.push([window[1].as_(), distance]);
    }

    result
}

/// Take a vector of nanosecond timestamp deltas, and scales them and returns the final unit
///
/// If the delta is very large, e.g. ~1s the vector is scaled to milliseconds and returns `ms`
/// and if the delta is very small (less than 1 microsecond) it's not scaled and returns `ns`.
pub fn scale_timestamp_distances(timestamp_delta_ns: &mut Vec<[f64; 2]>) -> String {
    // Skip the first sample as it's 0 per definition
    let first_samples: Vec<f64> = timestamp_delta_ns
        .iter()
        .skip(1)
        .take(20)
        .map(|s| s[1])
        .collect::<Vec<f64>>();
    let actual_len = first_samples.len();
    let sum: f64 = first_samples.iter().sum();
    let avg = sum / actual_len as f64;
    // If the distance is less than a micro-second we plot the distance in ns
    if avg < 1_000. {
        "ns".into()
    } else {
        for delta_ns in timestamp_delta_ns {
            delta_ns[1] /= 1_000_000.; // Convert to ms
        }
        "ms".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_empty() {
        let timestamps: Vec<i64> = vec![];
        let result = timestamp_distances(&timestamps);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single() {
        let timestamps = vec![1000i64];
        let result = timestamp_distances(&timestamps);
        assert_eq!(result, vec![[1000.0, 0.0]]);
    }

    #[test]
    fn test_multiple() {
        let timestamps = vec![1000i64, 1500, 2100, 2300];
        let result = timestamp_distances(&timestamps);
        assert_eq!(
            result,
            vec![
                [1000.0, 0.0],
                [1500.0, 500.0],
                [2100.0, 600.0],
                [2300.0, 200.0]
            ]
        );
    }

    #[test]
    fn test_i64() {
        let timestamps = vec![100i64, 150, 210];
        let result = timestamp_distances(&timestamps);
        assert_eq!(result, vec![[100.0, 0.0], [150.0, 50.0], [210.0, 60.0]]);
    }

    #[test]
    fn test_ndarray() {
        let timestamps = array![1000i64, 1500, 2100];
        let result = timestamp_distances_ndarray(&timestamps);
        assert_eq!(
            result,
            vec![[1000.0, 0.0], [1500.0, 500.0], [2100.0, 600.0]]
        );
    }

    #[test]
    fn test_ndarray_empty() {
        let timestamps: ndarray::Array1<i64> = array![];
        let result = timestamp_distances_ndarray(&timestamps);
        assert!(result.is_empty());
    }
}
