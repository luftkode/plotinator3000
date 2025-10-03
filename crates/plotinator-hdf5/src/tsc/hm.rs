use anyhow::{Context as _, ensure};
use ndarray::{Array1, Array2, Array3, Array4, ArrayView1, ArrayView2, ArrayView3, Axis, s};
use plotinator_log_if::prelude::{ExpectedPlotRange, RawPlot};
use rayon::{iter::ParallelIterator as _, slice::ParallelSlice as _};

use crate::tsc::metadata::RootMetadata;

struct CountTeslaVal(f64);
struct SubDivStrideNs(f64);
pub(crate) struct LastGateOn(pub usize);
struct GateCount(usize);

pub(crate) struct ZCoilZeroPositions(pub Vec<[f64; 2]>);
pub(crate) struct AllZCoilZeroPositions(pub Vec<Vec<[f64; 2]>>);

pub(crate) struct ZCoilBField(pub Vec<[f64; 2]>);
pub(crate) struct AllZCoilBField(pub Vec<Vec<[f64; 2]>>);

/// Helper function to calculate the cumulative sum of a 1D array view.
fn cumulative_sum_f64(array: &ArrayView1<f64>) -> Array1<f64> {
    let mut sum = 0.0;
    array
        .iter()
        .map(|&x| {
            sum += x;
            sum
        })
        .collect()
}

// NOTE: Don't bother optimizing this, never takes more than 1-4ms and typically takes around 500us
fn calc_subdivided_bfields(
    timevals: &Array1<u32>,
    signed_data_slice: &Array3<f64>,
    GateCount(gate_cnt): GateCount,
    LastGateOn(last_gate_on): LastGateOn,
    SubDivStrideNs(subdiv_stride_ns): SubDivStrideNs,
    CountTeslaVal(cnt_tesla_val): CountTeslaVal,
) -> anyhow::Result<(ZCoilZeroPositions, ZCoilBField)> {
    // mean over Axis(1) (the sign axis) -> shape [n_subdivisions, widths_len]
    let avg_per_subdiv = signed_data_slice
        .mean_axis(Axis(1))
        .ok_or_else(|| anyhow::anyhow!("Failed to average over sign axis"))?;

    // Validate widths length matches
    log::trace!(
        "avg_per_subdiv shape: {:?}, gate count: {gate_cnt}",
        avg_per_subdiv.shape(),
    );
    ensure!(
        avg_per_subdiv.shape()[1] == gate_cnt,
        "Shape mismatch: avg_per_subdiv width dim ({}) != gate count ({gate_cnt})",
        avg_per_subdiv.shape()[1]
    );

    let mut points_all_subdiv: Vec<[f64; 2]> =
        Vec::with_capacity(avg_per_subdiv.shape()[0] * gate_cnt);
    let mut zero_positions_this_t_idx = Vec::with_capacity(avg_per_subdiv.shape()[0]);

    for (i, subdiv_row) in avg_per_subdiv.outer_iter().enumerate() {
        let timeoffset: f64 = subdiv_stride_ns * i as f64;
        let cumsum_avgdat = cumulative_sum_f64(&subdiv_row.view());
        let plotdata = cumsum_avgdat.mapv(|v| v * cnt_tesla_val);

        zero_positions_this_t_idx.push([
            timevals[last_gate_on] as f64 + timeoffset,
            plotdata[last_gate_on] * 1e9,
        ]);

        // Combine into vector of [time (ns), value (nT)] points for this subdivision
        let subdiv_points = timevals
            .iter()
            .zip(plotdata.iter())
            .map(|(&t, &p)| [t as f64 + timeoffset, p * 1e9])
            .collect::<Vec<[f64; 2]>>();

        points_all_subdiv.extend(subdiv_points);
    }
    Ok((
        ZCoilZeroPositions(zero_positions_this_t_idx),
        ZCoilBField(points_all_subdiv),
    ))
}

/// Wrapper around the data from `/RX/monomial_basis_data/hm_gate_samples`
struct HmGateSamples {
    gate_widths: Array1<i32>,
}

impl HmGateSamples {
    // Parse from an open RX/mono
    fn parse_from_hm_h5(hdf5: &hdf5::File) -> anyhow::Result<Self> {
        let gate_samples_ds = hdf5.dataset("/RX/monomial_basis_data/hm_gate_samples")?;
        let gate_widths: Array1<i32> = gate_samples_ds
            .read_slice_1d(s![.., 0, 0])
            .context("failed slicing gate samples dataset")?;
        Ok(Self { gate_widths })
    }

    fn gate_count(&self) -> usize {
        self.gate_widths.len()
    }

    /// Relative gate times in ns
    #[inline]
    fn gate_sample_cumulative_time_sum(&self) -> Array1<u32> {
        const TIME_FACTOR: u32 = 200; // ns
        let mut sum: u32 = 0;
        self.gate_widths
            .view()
            .iter()
            .map(|v| {
                sum += *v as u32;
                sum * TIME_FACTOR
            })
            .collect()
    }
}

/// Wrapper around the data from `/RX/monomial_basis_data/hm_vm2`
struct HmVm2 {
    cnt_vm2: Array2<f64>,
}

impl HmVm2 {
    fn parse_from_h5(hdf5: &hdf5::File) -> anyhow::Result<Self> {
        let vm2_ds = hdf5.dataset("/RX/monomial_basis_data/hm_vm2")?;
        let cnt_vm2: Array2<f64> = vm2_ds
            .read_slice_2d(s![.., .., 0])
            .context("failed slicing vm2 dataset")?;
        Ok(Self { cnt_vm2 })
    }

    /// Returns the conversion value from count to tesla for a specific channel
    ///
    /// Channel 1/3/5 is coil Z/X/Y
    fn cnt_testa_val(&self, channel: usize) -> anyhow::Result<f64> {
        const SAMPLE_RATE: f64 = 5e6; // 5 MHz
        let cnt_tesla = self
            .cnt_vm2
            .get((0, channel))
            .ok_or_else(|| anyhow::anyhow!("failed to get cnt_vm2 value at [0, {channel}]"))?;
        Ok(cnt_tesla / SAMPLE_RATE)
    }
}

/// Wrapper around the data from `/RX/monomial_basis_data/hm_timestrides`
struct HmTimestrides {
    timestrides: Array1<f64>,
}

impl HmTimestrides {
    fn parse_from_h5(hdf5: &hdf5::File) -> anyhow::Result<Self> {
        let timestrides_dataset = hdf5.dataset("/RX/monomial_basis_data/hm_timestrides")?;
        let timestrides: Array1<f64> = timestrides_dataset
            .read()
            .context("failed to read timestrides dataset")?;
        log::debug!("{timestrides:?}");
        Ok(Self { timestrides })
    }

    // Returns the stride converted to nanoseconds for the subdivided HM data (highest resolution)
    fn subdivision_time_ns(&self) -> f64 {
        self.timestrides[1] * 1000.
    }
}

/// Wrapper around the data from `/RX/monomial_basis_data/hm_sign`
#[derive(Debug)]
struct HmSign {
    sign_data: Array4<i64>,
}

impl HmSign {
    fn parse_from_h5(hdf5: &hdf5::File) -> anyhow::Result<Self> {
        let sign_dataset = hdf5.dataset("/RX/monomial_basis_data/hm_sign")?;
        let sign_data: Array4<i64> = sign_dataset.read()?; // Actually stored as i32
        Ok(Self { sign_data })
    }

    // Reshape the signed dataset for broadcasting on the subdivided hm data for a single box and channel
    fn sign_reshaped_for_broadcast(&self) -> Array3<i64> {
        // Python: sign[..., 0, 0] -> [2, 1]
        let s: ArrayView2<i64> = self.sign_data.slice(s![.., .., 0, 0]);
        // Reshape sign for broadcasting: [2,1] -> [1,2,1]
        let s: ArrayView3<i64> = s.insert_axis(Axis(0));
        s.to_owned()
    }
}

/// Wrapper around the hm dataset from `/RX/monomial_basis_data/hm`
pub(crate) struct HmData<'h5> {
    h5file: &'h5 hdf5::File, // Keep file open for future access
    dataset_name: String,    // Path to dataset

    // The HM dataset is a 6-dimensional array: [N, 413, 2, 76, 6, 4]
    // Dimensions
    // [0]: Time in 1.1s resolution, aligned with GPS marks
    // [1]: Full resolution time
    // [2]: Half resolution (signed) (+/-)
    // [3]: ?
    // [4]: Channel, 1/3/5 is the production coils, 1 is Z
    // [5]: Sample methods, 1 is Box car, 2 is linear, 3 is hyperbolic(?), 4 is ?
    shape: [usize; 6], // Only load shape initially
}

impl<'h5> HmData<'h5> {
    pub fn from_hdf5(h5: &'h5 hdf5::File) -> hdf5::Result<Self> {
        let dataset_name = "/RX/monomial_basis_data/hm".to_owned();
        let dataset = h5.dataset(&dataset_name)?;
        let shape_vec = dataset.shape();
        let shape: [usize; 6] = shape_vec
            .try_into()
            .map_err(|e| hdf5::Error::Internal(format!("Dataset is not 6D: {e:?}")))?;

        Ok(Self {
            h5file: h5,
            dataset_name,
            shape,
        })
    }

    pub fn shape(&self) -> &[usize] {
        self.shape.as_ref()
    }

    /// Calculates B-field data
    ///
    /// # Arguments
    /// * `channel` - The channel index to use for the calculation. 1/3/5 is Z,X,Y coil
    /// * `box_idx` - The box index to use for the calculation
    /// * `last_gate_on` - is the gate number for the last gate of on time
    #[plotinator_proc_macros::log_time]
    pub fn calculate_b_field(
        &self,
        channel: usize,
        box_idx: usize,
        last_gate_on: usize,
    ) -> anyhow::Result<(AllZCoilZeroPositions, AllZCoilBField)> {
        // Read 'widths' from ".../hm_gate_samples"
        let gate_samples = HmGateSamples::parse_from_hm_h5(self.h5file)?;

        let hm_vm2 = HmVm2::parse_from_h5(self.h5file)?;
        let cnt_tesla_val = hm_vm2.cnt_testa_val(channel)?;

        // Read 'sign' data from ".../hm_sign"
        // purposefully converted to f64 to prevent doing that in the hot loop
        // it turns on multiplication that could be i64 to f64, but the precision loss is not measurable
        let sign_arr: Array3<f64> = HmSign::parse_from_h5(self.h5file)?
            .sign_reshaped_for_broadcast()
            .mapv(|v| v as f64);

        let subdivision_time_ns = HmTimestrides::parse_from_h5(self.h5file)?.subdivision_time_ns();

        let n_time = self.shape[0];

        // Precompute cumulative time offsets from widths (these are per-sample times)
        let timevals: ndarray::Array1<u32> = gate_samples.gate_sample_cumulative_time_sum();
        let gate_count = gate_samples.gate_count();

        let hm_dataset = self.h5file.dataset(&self.dataset_name)?;

        log::debug!(
            "Processing {n_time} periods of 1.1s HM data ({:.0}s)",
            n_time as f64 * 1.1
        );
        // Use chunked parallelism: process multiple iterations per thread
        // Each chunk should take a few milliseconds + I/O to amortize thread overhead
        const CHUNK_SIZE: usize = 32;

        let results: Vec<_> = (0..n_time)
            .collect::<Vec<_>>()
            .par_chunks(CHUNK_SIZE)
            .flat_map(|time_chunk| {
                let start_idx = time_chunk[0];
                let end_idx = *time_chunk.last().unwrap() + 1;

                // Read one large 4D chunk from HDF5 to reduce I/O sys calls
                let data_chunk_4d: Array4<f64> = hm_dataset
                    .read_slice(s![start_idx..end_idx, .., .., .., channel, box_idx])
                    .with_context(|| {
                        format!("Failed to read chunk for t_idx {start_idx}..{end_idx}")
                    })
                    .expect("failed reading chunk slice from HM dataset");

                // Iterate over the in-memory 4D chunk
                data_chunk_4d
                    .outer_iter()
                    .map(|data_slice_3d| {
                        // This could be multiplication between two i64 slices, but for optimization we use f64, the precision loss
                        // cannot be measured in our snapshots, so it's definitely good enough
                        let signed_data_slice: Array3<f64> = &data_slice_3d * &sign_arr;

                        calc_subdivided_bfields(
                            &timevals,
                            &signed_data_slice,
                            GateCount(gate_count),
                            LastGateOn(last_gate_on),
                            SubDivStrideNs(subdivision_time_ns),
                            CountTeslaVal(cnt_tesla_val),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Unpack results
        let (all_zero_positions, all_points): (Vec<_>, Vec<_>) = results
            .into_iter()
            .map(|(ZCoilZeroPositions(z), ZCoilBField(b))| (z, b))
            .unzip();

        Ok((
            AllZCoilZeroPositions(all_zero_positions),
            AllZCoilBField(all_points),
        ))
    }

    #[allow(
        dead_code,
        reason = "We will need this later, and want to test that the functionality doesn't break"
    )]
    /// Get a slice along the first dimension at fixed other coordinates
    /// Example: `get_slice_dim0([0, 1, 1, 1, 1])`
    pub fn get_slice_dim0(&self, coords: [usize; 5]) -> hdf5::Result<Array1<i64>> {
        let [dim1, dim2, dim3, dim4, dim5] = coords;
        let dataset = self.h5file.dataset(&self.dataset_name)?;
        dataset.read_slice_1d(s![.., dim1, dim2, dim3, dim4, dim5])
    }

    // Build metadata without loading full data
    #[allow(
        clippy::type_complexity,
        reason = "Lint is mostly triggered due to the metadata vector of tuple strings, which isn't that complex"
    )]
    #[plotinator_proc_macros::log_time]
    pub fn build_plots_and_metadata(
        &self,
        gps_timestamps: &[f64],
        root_metadata: &RootMetadata,
    ) -> anyhow::Result<(Vec<RawPlot>, Vec<(String, String)>)> {
        let hm_len = self.shape()[0];
        let gps_len = gps_timestamps.len();
        let metadata = vec![
            ("HM length".to_owned(), hm_len.to_string()),
            ("GPS length".to_owned(), gps_len.to_string()),
        ];

        let (AllZCoilZeroPositions(zero_positions_nested), AllZCoilBField(bfield_samples_nested)) =
            self.calculate_b_field(1, 1, root_metadata.last_gate_on_count())?;

        let mut final_bfield_points = Vec::with_capacity(bfield_samples_nested.len());
        let mut final_zero_points = Vec::with_capacity(zero_positions_nested.len());

        for ((mut b_samples, mut z_samples), gps_ts) in bfield_samples_nested
            .into_iter()
            .zip(zero_positions_nested.into_iter())
            .zip(gps_timestamps.iter())
        {
            // Apply timestamp to b-field points for this GPS mark
            for sample in &mut b_samples {
                sample[0] += gps_ts;
            }
            final_bfield_points.append(&mut b_samples);

            // Apply timestamp to zero-position points for this GPS mark
            for sample in &mut z_samples {
                sample[0] += gps_ts;
            }
            final_zero_points.append(&mut z_samples);
        }

        Ok((
            vec![
                RawPlot::new(
                    "0-position (Z) [nT]".to_owned(),
                    final_zero_points,
                    ExpectedPlotRange::Thousands,
                ),
                RawPlot::new(
                    "B-field (Z) [nT]".to_owned(),
                    final_bfield_points,
                    ExpectedPlotRange::Thousands,
                ),
            ],
            metadata,
        ))
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

        // Test accessing a slice
        let slice = hm_data.get_slice_dim0([0, 0, 0, 0, 0])?;
        println!("Slice along first dimension: {slice:?}");

        Ok(())
    }

    #[test]
    fn test_hm_data_dimensions() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;
        let hm_data = HmData::from_hdf5(&h5file)?;
        let root_metadata = RootMetadata::parse_from_tsc(&h5file)?;

        let gps_timestamps = vec![0.0, 1.0, 2.0, 3.0];

        let (_plots, metadata) =
            hm_data.build_plots_and_metadata(&gps_timestamps, &root_metadata)?;
        let shape = hm_data.shape();

        assert_eq!(metadata.len(), 2);

        // Verify expected shape [4, 413, 2, 76, 6, 4]
        assert_eq!(shape.len(), 6, "Should be 6-dimensional");
        assert_eq!(shape[0], 4, "First dimension should be 4");
        assert_eq!(shape[1], 413, "Second dimension should be 413");
        assert_eq!(shape[2], 2, "Third dimension should be 2");
        assert_eq!(shape[3], 76, "Fourth dimension should be 76");
        assert_eq!(shape[4], 6, "Fifth dimension should be 6");
        assert_eq!(shape[5], 4, "Sixth dimension should be 4");

        Ok(())
    }

    #[test]
    fn test_calculate_b_field_zero_pos_snapshot() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;
        let root_metadata = RootMetadata::parse_from_tsc(&h5file)?;
        let hm = HmData::from_hdf5(&h5file)?;

        // Define channel and box_idx for the test
        let channel = 1;
        let box_idx = 1;

        // Call the new function
        let (AllZCoilZeroPositions(zero_positions), _b_field_points) =
            hm.calculate_b_field(channel, box_idx, root_metadata.last_gate_on_count())?;

        let first_zvec = zero_positions.first().unwrap();
        assert_eq!(first_zvec.len(), 413);

        let first_10_zero_pos: Vec<[f64; 2]> = first_zvec.iter().copied().take(10).collect();

        insta::assert_debug_snapshot!(first_10_zero_pos);

        Ok(())
    }

    #[test]
    fn test_calculate_b_field_snapshot() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;
        let root_metadata = RootMetadata::parse_from_tsc(&h5file)?;
        let hm_data = HmData::from_hdf5(&h5file)?;

        // Define channel and box_idx for the test
        let channel = 1;
        let box_idx = 1;

        // Call the new function
        let (_zero_positions, AllZCoilBField(b_field_points)) =
            hm_data.calculate_b_field(channel, box_idx, root_metadata.last_gate_on_count())?;

        // Assert that the function produced a result
        assert!(
            !b_field_points.is_empty(),
            "B-field vector should not be empty"
        );

        let first_bfield_vec = b_field_points.first().unwrap();
        assert_eq!(first_bfield_vec.len(), 31388);

        let first_10_b_field: Vec<[f64; 2]> = first_bfield_vec.iter().copied().take(10).collect();

        insta::assert_debug_snapshot!(first_10_b_field);

        Ok(())
    }
}
