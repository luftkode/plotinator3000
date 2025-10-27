use std::io;

use chrono::{DateTime, TimeZone as _, Utc};
use hdf5::Dataset;
use ndarray::Array2;
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    stream_descriptor::StreamDescriptor,
    util::{assert_description_in_attrs, log_all_attributes, read_string_attribute},
};

const LEGEND_NAME: &str = "frame-TL";

impl SkytemHdf5 for FrameInclinometers {
    #[allow(
        clippy::too_many_lines,
        reason = "It's simple code that goes through all the datasets"
    )]
    fn from_path(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let h5file = hdf5::File::open(path)?;
        let (
            (angle1_dataset, attitude1_dataset, timestamp1_dataset),
            (angle2_dataset, attitude2_dataset, timestamp2_dataset),
            calibration_values_dataset,
        ) = Self::open_datasets(&h5file)?;

        assert_description_in_attrs(&angle1_dataset)?;
        log_all_attributes(&angle1_dataset);
        log_all_attributes(&attitude1_dataset);
        log_all_attributes(&timestamp1_dataset);
        log_all_attributes(&angle2_dataset);
        log_all_attributes(&attitude2_dataset);
        log_all_attributes(&timestamp2_dataset);
        log_all_attributes(&calibration_values_dataset);

        let angles1: Array2<f32> = angle1_dataset.read()?;
        let attitudes1: Array2<f32> = attitude1_dataset.read()?;
        let timestamps1: Array2<i64> = timestamp1_dataset.read_2d()?;

        let angles2: Array2<f32> = angle2_dataset.read()?;
        let attitudes2: Array2<f32> = attitude2_dataset.read()?;
        let timestamps2: Array2<i64> = timestamp2_dataset.read_2d()?;

        let mut raw_plots = vec![];
        let mut total_starting_timestamp = None;
        Self::build_plots(
            1,
            &mut total_starting_timestamp,
            &mut raw_plots,
            &timestamps1,
            &angles1,
            &attitudes1,
        );
        Self::build_plots(
            2,
            &mut total_starting_timestamp,
            &mut raw_plots,
            &timestamps2,
            &angles2,
            &attitudes2,
        );

        let metadata = Self::extract_metadata(
            &angle1_dataset,
            &attitude1_dataset,
            &timestamp1_dataset,
            &angle2_dataset,
            &attitude2_dataset,
            &timestamp2_dataset,
        )?;

        Ok(Self {
            starting_timestamp_utc: total_starting_timestamp.unwrap_or_default(),
            dataset_description: "Frame inclinometers".to_owned(),
            raw_plots,
            metadata,
        })
    }
}

impl Plotable for FrameInclinometers {
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
pub struct FrameInclinometers {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

type SensorDatasets = (Dataset, Dataset, Dataset);

const ANGLE1_DATASET: &str = "angle-1";
const ANGLE2_DATASET: &str = "angle-2";
const ATTITUDE1_DATASET: &str = "attitude-1";
const ATTITUDE2_DATASET: &str = "attitude-2";
const TIMESTAMP1_DATASET: &str = "timestamp-1";
const TIMESTAMP2_DATASET: &str = "timestamp-2";
pub(crate) const CALIBRATION_VALUES_DATASET: &str = "calibration-values";

const EXPECT_DIMENSION: usize = 2;
impl FrameInclinometers {
    fn open_datasets(h5file: &hdf5::File) -> io::Result<(SensorDatasets, SensorDatasets, Dataset)> {
        let angle1_dataset = crate::util::open_dataset(h5file, ANGLE1_DATASET, EXPECT_DIMENSION)?;
        let angle2_dataset = crate::util::open_dataset(h5file, ANGLE2_DATASET, EXPECT_DIMENSION)?;
        let attitude1_dataset =
            crate::util::open_dataset(h5file, ATTITUDE1_DATASET, EXPECT_DIMENSION)?;
        let attitude2_dataset =
            crate::util::open_dataset(h5file, ATTITUDE2_DATASET, EXPECT_DIMENSION)?;
        let timestamp1_dataset =
            crate::util::open_dataset(h5file, TIMESTAMP1_DATASET, EXPECT_DIMENSION)?;
        let timestamp2_dataset =
            crate::util::open_dataset(h5file, TIMESTAMP2_DATASET, EXPECT_DIMENSION)?;

        let calibration_values_dataset =
            crate::util::open_dataset(h5file, CALIBRATION_VALUES_DATASET, EXPECT_DIMENSION)?;

        Ok((
            (angle1_dataset, attitude1_dataset, timestamp1_dataset),
            (angle2_dataset, attitude2_dataset, timestamp2_dataset),
            calibration_values_dataset,
        ))
    }

    fn build_plots(
        sensor_id: u8,
        total_starting_timestamp: &mut Option<DateTime<Utc>>,
        rawplots: &mut Vec<RawPlot>,
        timestamps: &Array2<i64>,
        angles: &Array2<f32>,
        attitudes: &Array2<f32>,
    ) {
        if let Some((timestamps, first_timestamp)) = Self::process_timestamps(timestamps) {
            if let Some(total_starting_ts) = total_starting_timestamp {
                *total_starting_timestamp = Some(first_timestamp.min(*total_starting_ts));
            } else {
                *total_starting_timestamp = Some(first_timestamp);
            }

            let data_len = angles.nrows();

            let mut pitch_with_ts: Vec<[f64; 2]> = Vec::with_capacity(data_len);
            let mut old_roll_with_ts: Vec<[f64; 2]> = Vec::with_capacity(data_len);
            let mut roll_with_ts: Vec<[f64; 2]> = Vec::with_capacity(data_len);

            for ((angles_row, attitudes_row), timestamp) in angles
                .outer_iter()
                .zip(attitudes.outer_iter())
                .zip(timestamps.iter())
            {
                let pitch = attitudes_row[0];
                let roll = attitudes_row[1];

                // The old roll that is incorrectly calculated
                let old_roll = angles_row[1];

                let ts_float = *timestamp as f64;
                pitch_with_ts.push([ts_float, pitch as f64]);
                roll_with_ts.push([ts_float, roll as f64]);
                old_roll_with_ts.push([ts_float, old_roll as f64]);
            }

            rawplots.append(
                &mut RawPlotBuilder::new(format!("{LEGEND_NAME}{sensor_id}"))
                    .add_timestamp_delta(&timestamps)
                    .add(pitch_with_ts, DataType::Pitch)
                    .add(roll_with_ts, DataType::Roll)
                    .add(
                        old_roll_with_ts,
                        DataType::other_degrees("(Old) Roll".to_owned(), true),
                    )
                    .build(),
            );
        }
    }

    fn process_timestamps(timestamp_data: &Array2<i64>) -> Option<(Vec<i64>, DateTime<Utc>)> {
        let first = timestamp_data.first()?;
        let first_timestamp: DateTime<Utc> = chrono::Utc.timestamp_nanos(*first).to_utc();

        let mut timestamps = Vec::with_capacity(timestamp_data.nrows());
        for t in timestamp_data {
            timestamps.push(*t);
        }

        Some((timestamps, first_timestamp))
    }

    fn extract_metadata(
        angle1_ds: &Dataset,
        attitude1_ds: &Dataset,
        time1_ds: &Dataset,
        angle2_ds: &Dataset,
        attitude2_ds: &Dataset,
        time2_ds: &Dataset,
    ) -> io::Result<Vec<(String, String)>> {
        let angle1_ds_descr = read_string_attribute(&angle1_ds.attr("description")?)?;
        let angle1_stream_descr = StreamDescriptor::try_from(angle1_ds)?;
        let attitude1_ds_descr = read_string_attribute(&attitude1_ds.attr("description")?)?;
        let attitude1_stream_descr = StreamDescriptor::try_from(attitude1_ds)?;
        let timestamp1_ds_descr = read_string_attribute(&time1_ds.attr("description")?)?;
        let timestamp1_stream_descr = StreamDescriptor::try_from(time1_ds)?;

        let mut metadata = vec![("Dataset-1 Description".into(), angle1_ds_descr.clone())];
        metadata.extend_from_slice(&angle1_stream_descr.to_metadata());
        metadata.push(("Dataset-1 Description".to_owned(), attitude1_ds_descr));
        metadata.extend_from_slice(&attitude1_stream_descr.to_metadata());
        metadata.push(("Dataset-1 Description".to_owned(), timestamp1_ds_descr));
        metadata.extend_from_slice(&timestamp1_stream_descr.to_metadata());

        let angle2_ds_descr = read_string_attribute(&angle2_ds.attr("description")?)?;
        let angle2_stream_descr = StreamDescriptor::try_from(angle2_ds)?;
        let attitude2_ds_descr = read_string_attribute(&attitude2_ds.attr("description")?)?;
        let attitude2_stream_descr = StreamDescriptor::try_from(attitude2_ds)?;
        let timestamp2_ds_descr = read_string_attribute(&time2_ds.attr("description")?)?;
        let timestamp2_stream_descr = StreamDescriptor::try_from(time2_ds)?;

        metadata.push(("Dataset-2 Description".into(), angle2_ds_descr.clone()));
        metadata.extend_from_slice(&angle2_stream_descr.to_metadata());
        metadata.push(("Dataset-2 Description".into(), attitude2_ds_descr.clone()));
        metadata.extend_from_slice(&attitude2_stream_descr.to_metadata());
        metadata.push(("Dataset-2 Description".to_owned(), timestamp2_ds_descr));
        metadata.extend_from_slice(&timestamp2_stream_descr.to_metadata());

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::test_file_defs::frame_inclinometers::*;
    use testresult::TestResult;

    #[test]
    fn test_read_frame_inclinometers() -> TestResult {
        let frame_inclinometers = FrameInclinometers::from_path(frame_inclinometers())?;
        let metadata_count = frame_inclinometers.metadata.len();
        let raw_plot_count = frame_inclinometers.raw_plots.len();
        let first_raw_plot_points_count = match &frame_inclinometers.raw_plots[0] {
            RawPlot::Generic { common } => common.points().len(),
            RawPlot::GeoSpatialDataset(_) => unreachable!(),
        };

        let counts = vec![
            ("metadata_count", metadata_count),
            ("raw_plot_count", raw_plot_count),
            ("first_raw_plot_points_count", first_raw_plot_points_count),
        ];
        insta::assert_debug_snapshot!(counts);

        Ok(())
    }
}
