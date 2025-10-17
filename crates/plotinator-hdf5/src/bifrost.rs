use super::stream_descriptor::StreamDescriptor;
use anyhow::bail;
use chrono::Datelike as _;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use hdf5::Dataset;
use num_traits::ToPrimitive as _;
use plotinator_log_if::prelude::*;
use plotinator_log_if::rawplot::DataType;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

use crate::util::{
    assert_description_in_attrs, log_all_attributes, open_dataset, read_string_attribute,
};

const LEGEND_NAME: &str = "TX Bifrost";

impl SkytemHdf5 for BifrostLoopCurrent {
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let current_dataset = Self::open_bifrost_current_dataset(path)?;

        let dataset_description = read_string_attribute(&current_dataset.attr("description")?)?;
        let stream_descriptor_toml_str =
            read_string_attribute(&current_dataset.attr("stream_descriptor")?)?;
        let Ok(stream_descriptor): Result<StreamDescriptor, toml::de::Error> =
            toml::from_str(&stream_descriptor_toml_str)
        else {
            bail!(
                "Failed decoding 'stream_descriptor' string as TOML from stream_descriptor: {stream_descriptor_toml_str}"
            )
        };

        let data_3: ndarray::Array3<f32> = current_dataset.read()?;

        let (gps_timestamps, samples_per_ts, polarities) = data_3.dim();
        log::info!(
            "Got bifrost current dataset with: GPS timestamps: {gps_timestamps}, samples per timestamp: {samples_per_ts}, polarities: {polarities}"
        );
        let mut metadata = vec![
            ("Dataset Description".into(), dataset_description.clone()),
            ("GPS Timestamps".into(), gps_timestamps.to_string()),
            ("Samples per timestamp".into(), samples_per_ts.to_string()),
            ("Polarities".into(), polarities.to_string()),
        ];
        metadata.extend_from_slice(&stream_descriptor.to_metadata());
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

        let raw_plots = process_points_to_rawplots(starting_timestamp_ns, &data_3);

        Ok(Self {
            starting_timestamp_utc: first_january_this_year,
            dataset_description,
            raw_plots,
            metadata,
        })
    }
}

// Process the data points, meanings distributing as a timeseries and turning them into the raw plots that are compatible with the GUI
fn process_points_to_rawplots(
    starting_timestamp_ns: f64,
    current_data: &ndarray::Array3<f32>,
) -> Vec<RawPlot> {
    let (_, samples_per_ts, _) = current_data.dim();
    let mut polarity0_currents: Vec<[f64; 2]> = Vec::new();
    let mut polarity1_currents: Vec<[f64; 2]> = Vec::new();
    let mut combined_currents: Vec<[f64; 2]> = Vec::new();

    let nanosec_multiplier = 1_000_000_000.0;
    // Assumes one timestamp per second
    let sample_step_size_approx: f64 = 1.0 * nanosec_multiplier
        / (samples_per_ts
            .to_f64()
            .expect("Failed converting usize to f64"));
    let mut timestamp_idx = 0;
    let mut sample_idx = 0;

    for d in current_data.rows() {
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

        let mut p0 = [offset_ts, d[0].into()];
        let mut p1 = [offset_ts, d[1].into()];
        polarity0_currents.push(p0);
        polarity1_currents.push(p1);
        p0[1] = p0[1].abs();
        p1[0] += sample_step_size_approx / 2.;
        p1[1] = p1[1].abs();

        combined_currents.push(p0);
        combined_currents.push(p1);
    }

    let plot_polarity0 = RawPlotCommon::new(
        LEGEND_NAME,
        polarity0_currents,
        DataType::Current {
            suffix: Some("+".into()),
        },
    );
    let plot_polarity1 = RawPlotCommon::new(
        LEGEND_NAME,
        polarity1_currents,
        DataType::Current {
            suffix: Some("-".into()),
        },
    );
    let plot_combined = RawPlotCommon::new(
        LEGEND_NAME,
        combined_currents,
        DataType::Current {
            suffix: Some("Combined".into()),
        },
    );
    vec![
        plot_polarity0.into(),
        plot_polarity1.into(),
        plot_combined.into(),
    ]
}

impl Plotable for BifrostLoopCurrent {
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
pub struct BifrostLoopCurrent {
    // It does not actually contain a timestamp so we just add 1. january current year to make it slightly more convenient
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl BifrostLoopCurrent {
    pub const DATASET_NAME: &str = "hm_current";
    pub const DATASET_DIMENSIONS: usize = 3;
}

impl BifrostLoopCurrent {
    /// Opens the [`BifrostCurrent`] dataset and checks the validity of the [`Dataset`] structure.
    ///
    /// # Returns
    ///
    /// The [`BifrostCurrent`] dataset as a [`Dataset`].
    ///
    /// # Errors
    ///
    /// If opening the file or any validity check fails.
    pub fn open_bifrost_current_dataset(path: impl AsRef<Path>) -> io::Result<Dataset> {
        let hdf5_file = hdf5::File::open(&path)?;

        let current_data_set =
            open_dataset(&hdf5_file, Self::DATASET_NAME, Self::DATASET_DIMENSIONS)?;
        log_all_attributes(&current_data_set);

        assert_description_in_attrs(&current_data_set)?;

        Ok(current_data_set)
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    const TEST_DATA: &str = "../../test_data/hdf5/bifrost_current/20240930_100137_bifrost.h5";

    #[test]
    fn test_read_bifrost_current() -> TestResult {
        let bifrost_currents = BifrostLoopCurrent::from_path(TEST_DATA)?;
        assert_eq!(&bifrost_currents.dataset_description, "TX Loop Current");

        let expected_metadata = [
            ("Dataset Description".into(), "TX Loop Current".into()),
            ("GPS Timestamps".into(), "50".into()),
            ("Samples per timestamp".into(), "303".into()),
            ("Polarities".into(), "2".into()),
        ];

        for (metadata_kv, expected_metadata_kv) in bifrost_currents
            .metadata()
            .expect("Expected metadata but contained none")
            .iter()
            .zip(expected_metadata.iter())
        {
            assert_eq!(metadata_kv, expected_metadata_kv);
        }

        match &bifrost_currents.raw_plots()[1] {
            RawPlot::Generic { common } => {
                let expected_first_value = 4.434181213378906;
                assert_eq!(common.points().first().unwrap()[1], expected_first_value);

                let expected_last_value = 17.78993797302246;
                assert_eq!(common.points().last().unwrap()[1], expected_last_value);
            }
            RawPlot::GeoSpatialDataset(_) => unreachable!(),
        };

        Ok(())
    }
}
