use std::{io, path::Path};

use anyhow::bail;
use chrono::{DateTime, TimeZone as _, Utc};
use hdf5::Dataset;
use ndarray::Array2;
use plotinator_log_if::prelude::*;
use plotinator_ui_util::ExpectedPlotRange;
use serde::{Deserialize, Serialize};

use crate::{
    stream_descriptor::StreamDescriptor,
    util::{
        self, assert_description_in_attrs, log_all_attributes, open_dataset, read_string_attribute,
    },
};

const LEGEND_NAME_1: &str = "HE1";
const LEGEND_NAME_2: &str = "HE2";

impl SkytemHdf5 for FrameAltimeters {
    #[allow(clippy::too_many_lines, reason = "long but simple")]
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let (height1_dataset, timestamp1_dataset, height2_dataset, timestamp2_dataset) =
            Self::open_datasets(path)?;
        log_all_attributes(&height1_dataset);
        log_all_attributes(&timestamp1_dataset);
        log_all_attributes(&height2_dataset);
        log_all_attributes(&timestamp2_dataset);

        let heights1: Array2<f32> = height1_dataset.read_2d()?;
        let heights1: Vec<f64> = heights1.into_iter().map(|h| h as f64).collect();
        log::info!(
            "Got frame altimeter1 heights with {} samples",
            heights1.len()
        );
        let heights2: Array2<f32> = height2_dataset.read_2d()?;
        let heights2: Vec<f64> = heights2.into_iter().map(|h| h as f64).collect();
        log::info!(
            "Got frame altimeter2 heights with {} samples",
            heights2.len()
        );

        let timestamp1_data: Vec<i64> = timestamp1_dataset.read_raw()?;
        let timestamp2_data: Vec<i64> = timestamp2_dataset.read_raw()?;

        let first_timestamp_1 = timestamp1_data
            .first()
            .map(|ts| chrono::Utc.timestamp_nanos(*ts).to_utc());

        let first_timestamp_2 = timestamp2_data
            .first()
            .map(|ts| chrono::Utc.timestamp_nanos(*ts).to_utc());

        let total_starting_timestamp = match (first_timestamp_1, first_timestamp_2) {
            (None, None) => bail!("Both timestamp datasets are empty"),
            (Some(ts), None) | (None, Some(ts)) => ts,
            (Some(ts1), Some(ts2)) => ts1.min(ts2),
        };

        let metadata = Self::extract_metadata(
            &height1_dataset,
            &timestamp1_dataset,
            &height2_dataset,
            &timestamp2_dataset,
        )?;

        let mut raw_plots = vec![];

        let geo_data_builder1 = GeoSpatialDataBuilder::new(LEGEND_NAME_1)
            .timestamp(&timestamp1_data)
            .altitude_from_laser(heights1)
            .altitude_valid_range((0., Self::INVALID_VALUE_THRESHOLD));
        let geo_data_builder2 = GeoSpatialDataBuilder::new(LEGEND_NAME_2)
            .timestamp(&timestamp2_data)
            .altitude_from_laser(heights2)
            .altitude_valid_range((0., Self::INVALID_VALUE_THRESHOLD));

        if let Ok(Some(geo_data1)) = geo_data_builder1.build_into_rawplot() {
            raw_plots.push(geo_data1);
        }

        if let Ok(Some(geo_data2)) = geo_data_builder2.build_into_rawplot() {
            raw_plots.push(geo_data2);
        }

        Ok(Self {
            starting_timestamp_utc: total_starting_timestamp,
            dataset_description: "Frame altimeters".to_owned(),
            raw_plots,
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
    // The actual invalid placeholder value for the ILM is the ASCII string "99999.99"
    // we label any value above this value as invalid as the ILM cannot go above ~500m anyways
    const INVALID_VALUE_THRESHOLD: f64 = 2000.;

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
        assert_eq!(frame_altimeters.raw_plots.len(), 4);
        match &frame_altimeters.raw_plots[0] {
            RawPlot::Generic { .. } => unreachable!(),
            RawPlot::GeoSpatialDataset(geo_data) => assert_eq!(geo_data.len(), 1091),
        };

        Ok(())
    }
}
