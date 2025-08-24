use anyhow::{bail, ensure};
use ndarray::{ArrayBase, Dim, OwnedRepr};
use plotinator_log_if::prelude::RawPlot;

// 6-dimensional array: [N, 413, 2, 76, 6, 4]
type HmArray = ArrayBase<OwnedRepr<i64>, Dim<[usize; 6]>>;

/// Wrapper around the hm dataset from `/RX/monomial_basis_data/hm`
pub(crate) struct HmData {
    inner: HmArray,
}

impl HmData {
    pub fn from_hdf5(h5: &hdf5::File) -> hdf5::Result<Self> {
        let dataset = h5.dataset("/RX/monomial_basis_data/hm")?;
        let hm_data = dataset.read::<i64, ndarray::Dim<[usize; 6]>>()?;
        Ok(Self { inner: hm_data })
    }

    /// Get the shape of the dataset
    pub fn shape(&self) -> &[usize] {
        self.inner.shape()
    }

    #[allow(
        dead_code,
        reason = "We will need this later, and want to test that the functionality doesn't break"
    )]
    /// Create time-aligned point series from GPS timestamps
    /// Returns Vec<[f64; 2]> where each element is [timestamp, `hm_value`]
    /// The first dimension of hm data should match (or be 1 less than) GPS marks count
    pub fn create_time_series(
        &self,
        gps_timestamps: &[f64],
        coords: [usize; 5], // [dim1, dim2, dim3, dim4, dim5] - first dim varies with time
    ) -> anyhow::Result<Vec<[f64; 2]>> {
        let hm_len = self.shape()[0]; // First dimension length
        let gps_len = gps_timestamps.len();

        // Validate alignment: hm length should match GPS or be 1 less
        ensure!(
            hm_len == gps_len || hm_len == gps_len - 1,
            format!(
                "HM data length ({hm_len}) doesn't align with GPS timestamps ({gps_len}). Expected {gps_len} or {}",
                gps_len - 1
            )
        );

        let mut time_series = Vec::with_capacity(hm_len);
        let [dim1, dim2, dim3, dim4, dim5] = coords;

        for (i, gps_ts) in gps_timestamps.iter().enumerate().take(hm_len) {
            if let Some(hm_value) = self.get([i, dim1, dim2, dim3, dim4, dim5]) {
                time_series.push([*gps_ts, *hm_value as f64]);
            } else {
                bail!(
                    "Invalid HM coordinates at index {i}: [{i}, {dim1}, {dim2}, {dim3}, {dim4}, {dim5}]"
                );
            }
        }

        Ok(time_series)
    }

    #[allow(
        dead_code,
        reason = "We will need this later, and want to test that the functionality doesn't break"
    )]
    /// Access element at specific coordinates [dim0, dim1, dim2, dim3, dim4, dim5]
    /// Shape is [N, 413, 2, 76, 6, 4]
    pub fn get(&self, coords: [usize; 6]) -> Option<&i64> {
        self.inner.get(coords)
    }

    #[allow(
        dead_code,
        reason = "We will need this later, and want to test that the functionality doesn't break"
    )]
    /// Get a slice along the first dimension at fixed other coordinates
    /// Example: `get_slice_dim0([0, 1, 1, 1, 1])`
    pub fn get_slice_dim0(&self, coords: [usize; 5]) -> Option<ndarray::ArrayView1<i64>> {
        let [dim1, dim2, dim3, dim4, dim5] = coords;
        self.inner
            .slice(ndarray::s![.., dim1, dim2, dim3, dim4, dim5])
            .into_dimensionality()
            .ok()
    }

    /// Simply adds the HM and GPS lengths to the metadata
    pub fn build_plots_and_metadata(
        &self,
        gps_timestamps: &[f64],
    ) -> (Vec<RawPlot>, Vec<(String, String)>) {
        let mut metadata: Vec<(String, String)> = vec![];
        let hm_len = self.shape()[0]; // First dimension length
        let gps_len = gps_timestamps.len();
        metadata.push(("HM length".to_owned(), hm_len.to_string()));
        metadata.push(("GPS length".to_owned(), gps_len.to_string()));

        // Only metadata, TEM data is too complex
        (vec![], metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::test_file_defs::tsc::*;
    use testresult::TestResult;

    #[test]
    fn read_hm_data() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;
        let hm_data = HmData::from_hdf5(&h5file)?;

        println!("HM Data shape: {:?}", hm_data.shape());

        // Test accessing specific elements
        if let Some(value) = hm_data.get([0, 0, 0, 0, 0, 0]) {
            println!("Element [0,0,0,0,0,0]: {value}");
        }

        // Test accessing a slice
        if let Some(slice) = hm_data.get_slice_dim0([0, 0, 0, 0, 0]) {
            println!("Slice along first dimension: {slice:?}");
        }

        // Take a small sample for snapshot testing to avoid huge output
        let sample_coords = [
            [0, 0, 0, 0, 0, 0],
            [0, 1, 0, 0, 0, 0],
            [1, 0, 1, 0, 0, 0],
            [2, 10, 0, 10, 0, 0],
            [3, 100, 1, 50, 5, 3],
        ];

        let mut sample_values = Vec::new();
        for coords in sample_coords {
            if let Some(value) = hm_data.get(coords) {
                sample_values.push((coords, *value));
            }
        }

        insta::assert_debug_snapshot!(sample_values);
        Ok(())
    }

    #[test]
    fn test_hm_data_dimensions() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;
        let hm_data = HmData::from_hdf5(&h5file)?;

        let shape = hm_data.shape();

        // Verify expected shape [4, 413, 2, 76, 6, 4]
        assert_eq!(shape.len(), 6, "Should be 6-dimensional");
        assert_eq!(shape[0], 4, "First dimension should be 4");
        assert_eq!(shape[1], 413, "Second dimension should be 413");
        assert_eq!(shape[2], 2, "Third dimension should be 2");
        assert_eq!(shape[3], 76, "Fourth dimension should be 76");
        assert_eq!(shape[4], 6, "Fifth dimension should be 6");
        assert_eq!(shape[5], 4, "Sixth dimension should be 4");

        // Verify we can access boundary elements
        assert!(hm_data.get([3, 412, 1, 75, 5, 3]).is_some());
        assert!(hm_data.get([4, 0, 0, 0, 0, 0]).is_none()); // Out of bounds

        Ok(())
    }
}
