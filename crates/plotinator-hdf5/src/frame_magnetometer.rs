use std::{io, path::Path};

use chrono::{DateTime, TimeZone as _, Utc};
use hdf5::{Dataset, H5Type};
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    stream_descriptor::StreamDescriptor,
    util::{self, assert_description_in_attrs, log_all_attributes, read_string_attribute},
};

const LEGEND_NAME: &str = "frame-mag";

// Define the Magdata struct matching the HDF5 compound type
#[derive(Clone, Copy, Debug, H5Type)]
#[repr(C)]
pub struct Magdatah5 {
    /// System timestamp from when the data was received in UTC ns since the unix epoch.
    #[hdf5(rename = "sys-timestamp")]
    pub sys_timestamp: u64,
    /// The system time of the KMAG4 at the end of the measurement, in milliseconds
    #[hdf5(rename = "mag-sys-ts")]
    pub mag_timestamp: u64,
    /// The value of the measurement in tenth of pico tesla
    #[hdf5(rename = "value")]
    pub mag_value: u64,
}

// Define the gps-timestamp struct matching the HDF5 compound type
#[derive(Clone, Copy, Debug, H5Type)]
#[repr(C)]
pub struct GpsTimestamph5 {
    /// The mag system time at arrival of the PPS signal
    #[hdf5(rename = "sys-timestamp-at-pps")]
    pub sys_timestamp_at_pps: u64,
    /// GPS time in nanoseconds since the unix epoch
    #[hdf5(rename = "gps-timestamp")]
    pub gps_timestamp: u64,
}

impl SkytemHdf5 for FrameMagnetometer {
    const DESCRIPTIVE_NAME: &str = "Frame Magnetometer";

    #[allow(
        clippy::too_many_lines,
        reason = "It's simple code that goes through all the datasets"
    )]
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let h5file = hdf5::File::open(path)?;
        let magdata_ds: Dataset = util::open_dataset(&h5file, MAGDATA_DATASET, EXPECT_DIMENSIONS)?;
        let gps_timestamps_ds: Dataset =
            util::open_dataset(&h5file, GPS_TIMESTAMP_DATASET, EXPECT_DIMENSIONS)?;

        assert_description_in_attrs(&magdata_ds)?;
        log_all_attributes(&magdata_ds);
        log_all_attributes(&gps_timestamps_ds);
        let magdata: ndarray::Array2<Magdatah5> = magdata_ds.read()?;
        let gps_timestamps: ndarray::Array2<GpsTimestamph5> = gps_timestamps_ds.read()?;
        debug_assert_eq!(
            magdata.nrows(),
            gps_timestamps.nrows(),
            "Mismatch in dataset dimensions"
        );

        let data_len = magdata.nrows();

        let (timestamps, first_timestamp) = Self::process_timestamps(&magdata);

        let mut mag_vals: Vec<[f64; 2]> = Vec::with_capacity(data_len);
        let mut mag_clk_delta: Vec<[f64; 2]> = Vec::with_capacity(data_len);
        // Delta between the GPS timestamp and the system timestamp
        let mut gps_sys_delta: Vec<[f64; 2]> = Vec::with_capacity(data_len);
        // Delta between the first sys timestamp and the first mag timestamp
        // used to calculate clock drift
        let mut base_mag_sys_ts_delta: Option<i64> = None;

        for ((md_row, gps_row), ts) in magdata
            .outer_iter()
            .zip(gps_timestamps.outer_iter())
            .zip(timestamps)
        {
            let md = md_row[0];
            let gps = gps_row[0];

            // Store Mag values
            let tenth_of_pico_teslas: f64 = (md.mag_value as u32).into();
            let nano_t = tenth_of_pico_teslas / 10_000.;
            mag_vals.push([ts, nano_t]);

            // Store system time and mag system time delta
            let sys_ts: i64 = md.sys_timestamp as i64;
            let mag_ts: i64 = md.mag_timestamp as i64 * 1_000_000; // Convert from ms to ns
            let sys_mag_delta_ns = sys_ts - mag_ts;
            if let Some(base_sys_mag_delta_ns) = base_mag_sys_ts_delta {
                let delta_ns = base_sys_mag_delta_ns - sys_mag_delta_ns;
                let delta_ms = (delta_ns as f64) / 1_000_000.;
                mag_clk_delta.push([ts, delta_ms]);
            } else {
                base_mag_sys_ts_delta = Some(sys_mag_delta_ns);
                mag_clk_delta.push([ts, 0.]);
            }

            // Store system time and GPS time delta
            let gps_timestamp = gps.gps_timestamp;
            // It's set to 0 if this entry doesn't actually contain a GPS timestamp
            if gps_timestamp != 0 {
                let delta_ns: i64 = sys_ts - gps_timestamp as i64;
                let delta_ms: f64 = (delta_ns as f64) / 1_000_000.;
                gps_sys_delta.push([sys_ts as f64, delta_ms]);
            }
        }

        let raw_plots = vec![
            RawPlotCommon::new(LEGEND_NAME, mag_vals, DataType::MagneticFlux).into(),
            RawPlotCommon::new(
                LEGEND_NAME,
                mag_clk_delta,
                DataType::TimeDelta {
                    name: "Mag Clk".into(),
                    unit: "ms".into(),
                },
            )
            .into(),
            RawPlotCommon::new(
                LEGEND_NAME,
                gps_sys_delta,
                DataType::TimeDelta {
                    name: "GPS Time".into(),
                    unit: "ms".into(),
                },
            )
            .into(),
        ];

        let metadata = Self::extract_metadata(&magdata_ds, &gps_timestamps_ds)?;

        Ok(Self {
            starting_timestamp_utc: first_timestamp,
            dataset_description: "Frame Magnetometer".to_owned(),
            raw_plots,
            metadata,
        })
    }
}

impl Plotable for FrameMagnetometer {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.starting_timestamp_utc
    }

    fn descriptive_name(&self) -> &str {
        &self.dataset_description
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        Some(self.metadata.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMagnetometer {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

const MAGDATA_DATASET: &str = "mag-data";
const GPS_TIMESTAMP_DATASET: &str = "gps-timestamp";
const EXPECT_DIMENSIONS: usize = 2;

impl FrameMagnetometer {
    fn process_timestamps(
        timestamp_data: &ndarray::Array2<Magdatah5>,
    ) -> (Vec<f64>, DateTime<Utc>) {
        let first = timestamp_data.first().expect("Empty timestamps");
        let first_timestamp: DateTime<Utc> = chrono::Utc
            .timestamp_nanos(first.sys_timestamp as i64)
            .to_utc();

        let mut timestamps = vec![];
        // We use the sys timestamps as anchor for all the datasets
        for mag_data_row in timestamp_data.outer_iter() {
            let mag_data = mag_data_row[0];
            timestamps.push(mag_data.sys_timestamp as f64);
        }

        (timestamps, first_timestamp)
    }

    fn extract_metadata(
        magdata_ds: &Dataset,
        gps_timestamp_ds: &Dataset,
    ) -> io::Result<Vec<(String, String)>> {
        let magdata_ds_descr = read_string_attribute(&magdata_ds.attr("description")?)?;
        let magdata_stream_descr = StreamDescriptor::try_from(magdata_ds)?;
        let gps_ts_ds_descr = read_string_attribute(&gps_timestamp_ds.attr("description")?)?;
        let gps_ts_stream_descr = StreamDescriptor::try_from(gps_timestamp_ds)?;

        let mut metadata = vec![("Dataset Description".into(), magdata_ds_descr.clone())];
        metadata.extend_from_slice(&magdata_stream_descr.to_metadata());
        metadata.push(("Dataset Description".to_owned(), gps_ts_ds_descr));
        metadata.extend_from_slice(&gps_ts_stream_descr.to_metadata());

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::test_file_defs::frame_magnetometer::*;
    use testresult::TestResult;

    #[test]
    fn test_read_frame_magnetometer() -> TestResult {
        let frame_magnetometer = FrameMagnetometer::from_path(frame_magnetometer())?;
        assert_eq!(frame_magnetometer.metadata.len(), 22);
        assert_eq!(frame_magnetometer.raw_plots.len(), 3);
        match &frame_magnetometer.raw_plots[0] {
            RawPlot::Generic { common } => assert_eq!(common.points().len(), 46515),
            RawPlot::GeoSpatialDataset(_) => unreachable!(),
        };
        match &frame_magnetometer.raw_plots[1] {
            RawPlot::Generic { common } => assert_eq!(common.points().len(), 46515),
            RawPlot::GeoSpatialDataset(_) => unreachable!(),
        };
        match &frame_magnetometer.raw_plots[2] {
            RawPlot::Generic { common } => assert_eq!(common.points().len(), 930),
            RawPlot::GeoSpatialDataset(_) => unreachable!(),
        };

        Ok(())
    }
}
