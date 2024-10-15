use std::{io, path::Path};

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use hdf5::Datatype;
use log_if::log::SkytemLog;
use num_traits::ToPrimitive;

#[derive(Debug, Clone)]
pub struct BifrostCurrent {
    // It does not actually contain a timestamp so we just add 1. january current year to make it slightly more convenient
    starting_timestamp_ns: f64,
    polarity0_currents: Vec<[f64; 2]>,
    polarity1_currents: Vec<[f64; 2]>,
}

impl BifrostCurrent {
    pub const DATASET_NAME: &str = "hm_current";
    pub const DATASET_DIMENSIONS: usize = 3;
}

impl BifrostCurrent {
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let hdf5_file = hdf5::File::open(&path)?;
        let Ok(current_data_set) = hdf5_file.dataset(Self::DATASET_NAME) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "No {dataset_name} dataset in {fname}",
                    dataset_name = Self::DATASET_NAME,
                    fname = path.as_ref().display()
                ),
            ));
        };

        if current_data_set.ndim() != Self::DATASET_DIMENSIONS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Expected {ndim} dimensions in dataset {dataset_name}",
                    dataset_name = Self::DATASET_NAME,
                    ndim = Self::DATASET_DIMENSIONS
                ),
            ));
        }
        let data_3: ndarray::Array3<f32> = current_data_set.read()?;

        let (gps_timestamps, samples_per_ts, polarities) = data_3.dim();
        log::info!("Got bifrost current dataset with: GPS timestamps: {gps_timestamps}, samples per timestamp: {samples_per_ts}, polarities: {polarities}");

        // There's no timestamp (it has to be correlated with GPS data) so we just set the time to
        // 1. january current year, hopefully it will be obvious to the user that it is an auto generated timestamp and not the real one
        let now = chrono::offset::Local::now();
        let first_january_this_year =
            NaiveDate::from_ymd_opt(now.year(), 1, 1).expect("Invalid date");
        let first_january_this_year = NaiveDateTime::new(
            first_january_this_year,
            NaiveTime::from_hms_opt(0, 0, 0).expect("Invalid time"),
        )
        .and_utc();
        let starting_timestamp_ns = first_january_this_year
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range")
            .to_f64()
            .expect("Failed converting timestamp to f64");

        let mut polarity0_currents: Vec<[f64; 2]> = Vec::new();
        let mut polarity1_currents: Vec<[f64; 2]> = Vec::new();

        // Assumes one timestamp per second
        let sample_step_size_approx: f64 = 1.0
            / (samples_per_ts
                .to_f64()
                .expect("Failed converting usize to f64"));
        let mut timestamp_idx = 0;
        let mut sample_idx = 0;
        let nanosec_multiplier = 1_000_000_000.0;

        for d in data_3.rows() {
            let offset_from_start = (timestamp_idx as f64) * nanosec_multiplier
                + (sample_idx as f64) * sample_step_size_approx;

            let offset_ts = starting_timestamp_ns + offset_from_start;

            sample_idx += 1;
            if sample_idx == samples_per_ts {
                sample_idx = 0;
                timestamp_idx += 1;
            }
            if d.len() != 2 {
                log::error!(
                    "Expected bifrost hm_current row length of 2, got: {}",
                    d.len()
                );
                continue;
            }

            if let Some(current) = d.get(0) {
                let current: f64 = (*current).into();
                polarity0_currents.push([offset_ts, current]);
            }
            if let Some(current) = d.get(1) {
                let current: f64 = (*current).into();
                polarity1_currents.push([offset_ts, current]);
            }
        }

        Ok(Self {
            starting_timestamp_ns,
            polarity0_currents,
            polarity1_currents,
        })
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use crate::util::NativePrimitive;

    use super::*;

    const TEST_DATA: &str = "../../test_data/hdf5/bifrost_current/20240930_100137_bifrost.h5";

    #[test]
    fn test_read_bifrost_current() -> TestResult {
        let bifrost_currents = BifrostCurrent::from_path(TEST_DATA)?;

        eprintln!("{bifrost_currents:?}");

        Ok(())
    }
}
