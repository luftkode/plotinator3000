use chrono::TimeZone as _;
use chrono::{DateTime, Utc};
use hdf5::{Dataset, H5Type};
use ndarray::Array2;
use plotinator_log_if::prelude::*;
use plotinator_ui_util::ExpectedPlotRange;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

use crate::stream_descriptor::StreamDescriptor;
use crate::util::{
    self, assert_description_in_attrs, log_all_attributes, open_dataset,
    read_any_attribute_to_string, read_string_attribute,
};

const RAW_PLOT_NAME_SUFFIX: &str = "(Njord-WASP)";

impl SkytemHdf5 for Wasp200 {
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let (height_dataset, timestamp_dataset) = Self::open_wasp200_datasets(path)?;
        log_all_attributes(&height_dataset);
        log_all_attributes(&timestamp_dataset);

        let height_unit = read_any_attribute_to_string(&height_dataset.attr("unit")?)?;
        let heights: ndarray::Array2<f32> = height_dataset.read()?;
        log::info!("Got wasp wasp heights with {} samples", heights.len());

        let (timestamps, first_timestamp, delta_t_samples_opt) =
            Self::extract_timestamps(&timestamp_dataset)?;

        let mut height_with_ts: Vec<[f64; 2]> = Vec::new();

        for (height, ts) in heights.iter().zip(timestamps) {
            height_with_ts.push([ts, *height as f64]);
        }

        let mut raw_plots = vec![
            RawPlotCommon::new(
                format!("Height [{height_unit}] {RAW_PLOT_NAME_SUFFIX}"),
                height_with_ts,
                ExpectedPlotRange::Hundreds,
            )
            .into(),
        ];

        if let Some(delta_t_samples) = delta_t_samples_opt {
            raw_plots.push(delta_t_samples.into());
        }

        let metadata = Self::extract_metadata(&height_dataset, &timestamp_dataset)?;

        Ok(Self {
            starting_timestamp_utc: first_timestamp,
            dataset_description: "Njord Wasp200 Height".to_owned(),
            raw_plots,
            metadata,
        })
    }
}

impl Plotable for Wasp200 {
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

#[derive(H5Type, Clone, Debug)]
#[repr(C)]
struct Timestamp {
    time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wasp200 {
    // It does not actually contain a timestamp so we just add 1. january current year to make it slightly more convenient
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl Wasp200 {
    const HEIGHT_DATASET: &str = "height";
    const TIMESTAMP_DATASET: &str = "timestamp";
    const EXPECT_DIMENSION: usize = 2;

    fn open_wasp200_datasets(path: impl AsRef<Path>) -> io::Result<(Dataset, Dataset)> {
        let hdf5_file = hdf5::File::open(&path)?;

        let height_dataset =
            open_dataset(&hdf5_file, Self::HEIGHT_DATASET, Self::EXPECT_DIMENSION)?;

        assert_description_in_attrs(&height_dataset)?;

        let timestamp_dataset =
            open_dataset(&hdf5_file, Self::TIMESTAMP_DATASET, Self::EXPECT_DIMENSION)?;
        Ok((height_dataset, timestamp_dataset))
    }

    fn extract_metadata(
        height_dataset: &Dataset,
        time_dataset: &Dataset,
    ) -> io::Result<Vec<(String, String)>> {
        let height_dataset_description =
            read_string_attribute(&height_dataset.attr("description")?)?;
        let height_stream_descriptor = StreamDescriptor::try_from(height_dataset)?;
        let timestamp_dataset_description =
            read_string_attribute(&time_dataset.attr("description")?)?;
        let timestamp_stream_descriptor = StreamDescriptor::try_from(time_dataset)?;

        let mut metadata = vec![(
            "Dataset Description".into(),
            height_dataset_description.clone(),
        )];
        metadata.extend_from_slice(&height_stream_descriptor.to_metadata());
        metadata.push((
            "Dataset Description".to_owned(),
            timestamp_dataset_description,
        ));
        metadata.extend_from_slice(&timestamp_stream_descriptor.to_metadata());

        Ok(metadata)
    }

    fn extract_timestamps(
        timestamp_dataset: &hdf5::Dataset,
    ) -> anyhow::Result<(Vec<f64>, DateTime<Utc>, Option<RawPlotCommon>)> {
        let timestamps_raw: Vec<i64> = read_timestamps(timestamp_dataset)?;
        let first_timestamp: DateTime<Utc> = chrono::Utc
            .timestamp_nanos(*timestamps_raw.first().expect("Empty timestamps"))
            .to_utc();

        let delta_t_samples =
            util::gen_time_between_samples_rawplot(&timestamps_raw, RAW_PLOT_NAME_SUFFIX);

        let mut timestamps = vec![];
        for t in timestamps_raw {
            timestamps.push(t as f64);
        }

        Ok((timestamps, first_timestamp, delta_t_samples))
    }
}

fn read_timestamps(timestamp_dataset: &hdf5::Dataset) -> anyhow::Result<Vec<i64>> {
    let timestamps: Vec<i64> = match timestamp_dataset.read_raw() {
        Ok(t) => t,
        Err(e) => {
            log::warn!(
                "Failed reading Njord altimeter timestamp dataset as simple i64: {e} - trying as compound data type"
            );
            // Read timestamp as 2D array of `Timestamp` structs, then extract `.time`
            let timestamp_data: Array2<Timestamp> = timestamp_dataset.read_2d::<Timestamp>()?;
            timestamp_data.iter().map(|t| t.time).collect()
        }
    };

    Ok(timestamps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::test_file_defs::wasp200::*;
    use testresult::TestResult;

    #[test]
    fn test_read_wasp200_height() -> TestResult {
        let wasp200 = Wasp200::from_path(wasp200())?;

        match &wasp200.raw_plots[0] {
            RawPlot::Generic { common } => assert_eq!(common.points().len(), 70971),
            _ => unreachable!(),
        }

        Ok(())
    }
}
