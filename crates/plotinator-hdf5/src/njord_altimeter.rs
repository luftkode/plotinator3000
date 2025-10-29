use anyhow::bail;
use chrono::TimeZone as _;
use chrono::{DateTime, Utc};
use hdf5::{Dataset, H5Type};
use ndarray::Array2;
use plotinator_log_if::prelude::*;
use plotinator_log_if::rawplot::path_data::GeoSpatialDataset;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

use crate::stream_descriptor::StreamDescriptor;
use crate::util::{
    assert_description_in_attrs, log_all_attributes, open_dataset, read_string_attribute,
};

const LEGEND_PREFIX: &str = "Njord";

impl SkytemHdf5 for NjordAltimeter {
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = hdf5::File::open(&path)?;
        let mut raw_plots = vec![];
        let mut metadata = vec![];

        let mut starting_timestamp = None;

        match Self::read_wasp200_plot_data(&file) {
            Ok(AltimeterSensorData {
                data,
                metadata: mut wasp_metadata,
                first_timestamp,
            }) => {
                starting_timestamp = Some(first_timestamp);
                raw_plots.push(data.into());
                metadata.append(&mut wasp_metadata);
            }
            Err(e) => {
                log::warn!("Could not read Njord Altimeter WASP200 dataset: {e}");
            }
        }

        match Self::read_sf20_plot_data(&file) {
            Ok(AltimeterSensorData {
                data,
                metadata: mut sf20_metadata,
                first_timestamp,
            }) => {
                if let Some(ts) = starting_timestamp {
                    if first_timestamp < ts {
                        starting_timestamp = Some(first_timestamp);
                    }
                } else {
                    starting_timestamp = Some(first_timestamp);
                }
                raw_plots.push(data.into());
                metadata.append(&mut sf20_metadata);
            }
            Err(e) => log::warn!("Could not read Njord altimeter SF20 dataset: {e}"),
        }

        let Some(starting_timestamp_utc) = starting_timestamp else {
            bail!("No valid Njord Altimeter datasets");
        };

        Ok(Self {
            starting_timestamp_utc,
            dataset_description: "Njord Altimeter".to_owned(),
            raw_plots,
            metadata,
        })
    }
}

impl Plotable for NjordAltimeter {
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

// The data from the HDF5 file for a single sensor type
struct AltimeterSensorData {
    data: GeoSpatialDataset,
    metadata: Vec<(String, String)>,
    first_timestamp: DateTime<Utc>,
}

// Old wasp200 datasets unfortunately had this compound datatype as timestamps
#[derive(H5Type, Clone, Debug)]
#[repr(C)]
struct Timestamp {
    time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NjordAltimeter {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl NjordAltimeter {
    const WASP200_HEIGHT_DATASET: &str = "height";
    const WASP200_TIMESTAMP_DATASET: &str = "timestamp";
    const EXPECT_DIMENSION: usize = 2;
    const SF20_HEIGHT_DATASET: &str = "sf20_height";
    const SF20_TIMESTAMP_DATASET: &str = "sf20_timestamp";

    fn read_wasp200_plot_data(file: &hdf5::File) -> anyhow::Result<AltimeterSensorData> {
        let (height_dataset, timestamp_dataset) = Self::open_wasp200_datasets(file)?;
        log_all_attributes(&height_dataset);
        log_all_attributes(&timestamp_dataset);

        let heights: Vec<f32> = height_dataset.read_raw()?;
        let heights: Vec<f64> = heights.into_iter().map(|h| h as f64).collect();
        log::info!(
            "Got Njord Altimeter Wasp heights with {} samples",
            heights.len()
        );

        let (timestamps, first_timestamp) = Self::extract_wasp200_timestamps(&timestamp_dataset)?;

        let Some(data) = GeoSpatialDataBuilder::new(format!("{LEGEND_PREFIX}-WASP"))
            .timestamp(&timestamps)
            .altitude_from_laser(heights)
            .build()
            .expect("invalid builder")
        else {
            anyhow::bail!("Empty dataset")
        };
        let metadata = Self::extract_metadata(&height_dataset, &timestamp_dataset)?;
        Ok(AltimeterSensorData {
            data,
            metadata,
            first_timestamp,
        })
    }

    fn open_wasp200_datasets(file: &hdf5::File) -> io::Result<(Dataset, Dataset)> {
        let heights = open_dataset(file, Self::WASP200_HEIGHT_DATASET, Self::EXPECT_DIMENSION)?;
        assert_description_in_attrs(&heights)?;
        let timestamps = open_dataset(
            file,
            Self::WASP200_TIMESTAMP_DATASET,
            Self::EXPECT_DIMENSION,
        )?;
        Ok((heights, timestamps))
    }

    fn read_sf20_plot_data(file: &hdf5::File) -> anyhow::Result<AltimeterSensorData> {
        let ds_height = open_dataset(file, Self::SF20_HEIGHT_DATASET, Self::EXPECT_DIMENSION)?;
        let ds_time = open_dataset(file, Self::SF20_TIMESTAMP_DATASET, Self::EXPECT_DIMENSION)?;

        let heights: Vec<f64> = ds_height
            .read_raw::<f32>()?
            .into_iter()
            .map(|h| h as f64)
            .collect();
        log::info!(
            "Got Njord Altimeter SF20 heights with {} samples",
            heights.len()
        );

        let timestamps: Vec<i64> = ds_time.read_raw()?;
        let Some(first_ts) = timestamps.first() else {
            bail!("Empty timestamps dataset");
        };
        let first_timestamp = Utc.timestamp_nanos(*first_ts).to_utc();

        let Some(data) = GeoSpatialDataBuilder::new(format!("{LEGEND_PREFIX}-SF20"))
            .timestamp(&timestamps)
            .altitude_from_laser(heights)
            .build()
            .expect("invalid builder")
        else {
            bail!("Empty dataset");
        };

        let metadata = Self::extract_metadata(&ds_height, &ds_time)?;
        Ok(AltimeterSensorData {
            data,
            metadata,
            first_timestamp,
        })
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
        let sample_count = time_dataset.size();
        let mut metadata = vec![
            (
                "Dataset Description".into(),
                height_dataset_description.clone(),
            ),
            ("Samples".into(), format!("{sample_count}")),
        ];
        metadata.extend_from_slice(&height_stream_descriptor.to_metadata());
        metadata.push((
            "Dataset Description".to_owned(),
            timestamp_dataset_description,
        ));
        metadata.extend_from_slice(&timestamp_stream_descriptor.to_metadata());

        Ok(metadata)
    }

    fn extract_wasp200_timestamps(
        timestamp_dataset: &hdf5::Dataset,
    ) -> anyhow::Result<(Vec<i64>, DateTime<Utc>)> {
        let timestamps: Vec<i64> = read_wasp200_timestamps(timestamp_dataset)?;
        let first_timestamp: DateTime<Utc> = chrono::Utc
            .timestamp_nanos(*timestamps.first().expect("Empty timestamps"))
            .to_utc();

        Ok((timestamps, first_timestamp))
    }
}

fn read_wasp200_timestamps(timestamp_dataset: &hdf5::Dataset) -> anyhow::Result<Vec<i64>> {
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
    use plotinator_test_util::test_file_defs::njord_altimeter::*;
    use testresult::TestResult;

    #[test]
    fn test_read_wasp200_height() -> TestResult {
        let wasp200 = NjordAltimeter::from_path(njord_altimeter_wasp200())?;

        match &wasp200.raw_plots[0] {
            RawPlot::Generic { .. } => unreachable!(),
            RawPlot::GeoSpatialDataset(data) => assert_eq!(data.len(), 70971),
        }

        Ok(())
    }

    #[test]
    fn test_read_njord_altimeter_wasp200_and_sf20_height() -> TestResult {
        let wasp200 = NjordAltimeter::from_path(njord_altimeter_wasp200_sf20())?;

        match &wasp200.raw_plots[0] {
            RawPlot::Generic { .. } => unreachable!(),
            RawPlot::GeoSpatialDataset(data) => {
                assert_eq!(data.name(), "Njord-WASP");
                assert_eq!(data.len(), 7795);
            }
        }
        match &wasp200.raw_plots[1] {
            RawPlot::Generic { .. } => unreachable!(),
            RawPlot::GeoSpatialDataset(data) => {
                assert_eq!(data.name(), "Njord-SF20");
                assert_eq!(data.len(), 7796);
            }
        }

        Ok(())
    }
}
