use std::{io, path::Path};

use chrono::{DateTime, TimeZone as _, Utc};
use hdf5::Dataset;
use plotinator_log_if::{hdf5::SkytemHdf5, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    stream_descriptor::StreamDescriptor,
    util::{
        assert_description_in_attrs, log_all_attributes, open_dataset,
        read_any_attribute_to_string, read_string_attribute,
    },
};

impl SkytemHdf5 for FrameAltimeters {
    fn from_path(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let (height1_dataset, timestamp1_dataset, height2_dataset, timestamp2_dataset) =
            Self::open_datasets(path)?;
        log_all_attributes(&height1_dataset);
        log_all_attributes(&timestamp1_dataset);
        log_all_attributes(&height2_dataset);
        log_all_attributes(&timestamp2_dataset);

        let height1_unit = read_any_attribute_to_string(&height1_dataset.attr("unit")?)?;
        let heights1: ndarray::Array2<f32> = height1_dataset.read()?;
        log::info!(
            "Got frame altimeter1 heights with {} samples",
            heights1.len()
        );
        let height2_unit = read_any_attribute_to_string(&height2_dataset.attr("unit")?)?;
        let heights2: ndarray::Array2<f32> = height2_dataset.read()?;
        log::info!(
            "Got frame altimeter1 heights with {} samples",
            heights2.len()
        );

        let timestamp1_data: ndarray::Array2<i64> = timestamp1_dataset.read_2d()?;
        let timestamp2_data: ndarray::Array2<i64> = timestamp2_dataset.read_2d()?;

        let (timestamps1, first_timestamp1) = Self::process_timestamps(&timestamp1_data);
        let (timestamps2, first_timestamp2) = Self::process_timestamps(&timestamp2_data);
        let total_starting_timestamp = std::cmp::min(first_timestamp1, first_timestamp2);

        let mut height1_with_ts: Vec<[f64; 2]> = Vec::new();
        for (height, ts) in heights1.iter().zip(timestamps1) {
            height1_with_ts.push([ts, *height as f64]);
        }
        let mut height2_with_ts: Vec<[f64; 2]> = Vec::new();
        for (height, ts) in heights2.iter().zip(timestamps2) {
            height2_with_ts.push([ts, *height as f64]);
        }

        let rawplot1 = RawPlot::new(
            format!("Height-1 [{height1_unit}]"),
            height1_with_ts,
            ExpectedPlotRange::OneToOneHundred,
        );
        let rawplot2 = RawPlot::new(
            format!("Height-2 [{height2_unit}]"),
            height2_with_ts,
            ExpectedPlotRange::OneToOneHundred,
        );

        let metadata = Self::extract_metadata(
            &height1_dataset,
            &timestamp1_dataset,
            &height2_dataset,
            &timestamp2_dataset,
        )?;

        Ok(Self {
            starting_timestamp_utc: total_starting_timestamp,
            dataset_description: "Frame altimeters".to_owned(),
            raw_plots: vec![rawplot1, rawplot2],
            metadata,
        })
    }
}

impl Plotable for FrameAltimeters {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.starting_timestamp_utc
    }

    fn descriptive_name(&self) -> &str {
        &self.dataset_description
    }

    fn labels(&self) -> Option<&[plotinator_log_if::prelude::PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        Some(self.metadata.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameAltimeters {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl FrameAltimeters {
    const HEIGHT1_DATASET: &str = "height-1";
    const HEIGHT2_DATASET: &str = "height-2";
    const TIMESTAMP1_DATASET: &str = "timestamp-1";
    const TIMESTAMP2_DATASET: &str = "timestamp-2";
    const EXPECT_DIMENSION: usize = 2;

    fn open_datasets(path: impl AsRef<Path>) -> io::Result<(Dataset, Dataset, Dataset, Dataset)> {
        let hdf5_file = hdf5::File::open(&path)?;

        let height1_dataset =
            open_dataset(&hdf5_file, Self::HEIGHT1_DATASET, Self::EXPECT_DIMENSION)?;
        assert_description_in_attrs(&height1_dataset)?;

        let height2_dataset =
            open_dataset(&hdf5_file, Self::HEIGHT2_DATASET, Self::EXPECT_DIMENSION)?;

        let timestamp1_dataset =
            open_dataset(&hdf5_file, Self::TIMESTAMP1_DATASET, Self::EXPECT_DIMENSION)?;

        let timestamp2_dataset =
            open_dataset(&hdf5_file, Self::TIMESTAMP2_DATASET, Self::EXPECT_DIMENSION)?;

        Ok((
            height1_dataset,
            timestamp1_dataset,
            height2_dataset,
            timestamp2_dataset,
        ))
    }

    fn process_timestamps(timestamp_data: &ndarray::Array2<i64>) -> (Vec<f64>, DateTime<Utc>) {
        let first = timestamp_data.first().expect("Empty timestamps");
        let first_timestamp: DateTime<Utc> = chrono::Utc.timestamp_nanos(*first).to_utc();

        let mut timestamps = vec![];
        for t in timestamp_data {
            timestamps.push(*t as f64);
        }

        (timestamps, first_timestamp)
    }

    fn extract_metadata(
        height1_dataset: &Dataset,
        time1_dataset: &Dataset,
        height2_dataset: &Dataset,
        time2_dataset: &Dataset,
    ) -> io::Result<Vec<(String, String)>> {
        let height1_dataset_description =
            read_string_attribute(&height1_dataset.attr("description")?)?;
        let height1_stream_descriptor = StreamDescriptor::try_from(height1_dataset)?;
        let timestamp1_dataset_description =
            read_string_attribute(&time1_dataset.attr("description")?)?;
        let timestamp1_stream_descriptor = StreamDescriptor::try_from(time1_dataset)?;

        let mut metadata = vec![(
            "Dataset-1 Description".into(),
            height1_dataset_description.clone(),
        )];
        metadata.extend_from_slice(&height1_stream_descriptor.to_metadata());
        metadata.push((
            "Dataset-1 Description".to_owned(),
            timestamp1_dataset_description,
        ));
        metadata.extend_from_slice(&timestamp1_stream_descriptor.to_metadata());

        let height2_dataset_description =
            read_string_attribute(&height2_dataset.attr("description")?)?;
        let height2_stream_descriptor = StreamDescriptor::try_from(height2_dataset)?;
        let timestamp2_dataset_description =
            read_string_attribute(&time2_dataset.attr("description")?)?;
        let timestamp2_stream_descriptor = StreamDescriptor::try_from(time2_dataset)?;

        metadata.push((
            "Dataset-2 Description".into(),
            height2_dataset_description.clone(),
        ));
        metadata.extend_from_slice(&height2_stream_descriptor.to_metadata());
        metadata.push((
            "Dataset-2 Description".to_owned(),
            timestamp2_dataset_description,
        ));
        metadata.extend_from_slice(&timestamp2_stream_descriptor.to_metadata());

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::test_file_defs::frame_altimeters::*;
    use testresult::TestResult;

    #[test]
    fn test_read_frame_altimeters() -> TestResult {
        let frame_altimeters = FrameAltimeters::from_path(frame_altimeters())?;
        assert_eq!(frame_altimeters.metadata.len(), 48);
        assert_eq!(frame_altimeters.raw_plots.len(), 2);
        assert_eq!(frame_altimeters.raw_plots[0].points().len(), 1091);

        Ok(())
    }
}
