use chrono::TimeZone;
use chrono::{DateTime, Utc};
use hdf5::{Dataset, H5Type};
use log_if::prelude::*;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

use crate::stream_descriptor::StreamDescriptor;
use crate::util::{read_any_attribute_to_string, read_string_attribute};

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
    pub fn open_wasp200_datasets(path: impl AsRef<Path>) -> io::Result<(Dataset, Dataset)> {
        let hdf5_file = hdf5::File::open(&path)?;

        let height_dataset_name = "height";
        let expect_height_dataset_ndim = 2;
        let Ok(height_dataset) = hdf5_file.dataset(height_dataset_name) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "No {height_dataset_name} dataset in {fname}",
                    fname = path.as_ref().display()
                ),
            ));
        };

        if height_dataset.ndim() != expect_height_dataset_ndim {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Expected {expect_height_dataset_ndim} dimensions in dataset {height_dataset_name}"
                ),
            ));
        }

        let dataset_attributes = height_dataset.attr_names()?;

        if !dataset_attributes.contains(&"description".to_owned()) {
            let comma_separated_attr_list = dataset_attributes
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Expected 'description' among dataset attributes, but attributes do not contain 'description'. Attributes in dataset: {comma_separated_attr_list}",
                ),
            ));
        }

        let timestamp_dataset_name = "timestamp";
        let expect_timestamp_dataset_ndim = 2;
        let Ok(timestamp_dataset) = hdf5_file.dataset(timestamp_dataset_name) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "No {timestamp_dataset_name} dataset in {fname}",
                    fname = path.as_ref().display()
                ),
            ));
        };

        if timestamp_dataset.ndim() != expect_timestamp_dataset_ndim {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Expected {expect_timestamp_dataset_ndim} dimensions in dataset {timestamp_dataset_name}"
                ),
            ));
        }
        Ok((height_dataset, timestamp_dataset))
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let (height_dataset, timestamp_dataset) = Self::open_wasp200_datasets(path)?;

        let height_dataset_description =
            read_string_attribute(&height_dataset.attr("description")?)?;
        let height_stream_descriptor_toml_str =
            read_string_attribute(&height_dataset.attr("stream_descriptor")?)?;
        let Ok(height_stream_descriptor): Result<StreamDescriptor, toml::de::Error> =
            toml::from_str(&height_stream_descriptor_toml_str)
        else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed decoding 'stream_descriptor' string as TOML from stream_descriptor: {height_stream_descriptor_toml_str}"
                ),
            ));
        };

        for a in height_dataset.attr_names()? {
            let attr = height_dataset.attr(&a)?;
            let attr_val_as_string = read_any_attribute_to_string(&attr)?;
            log::info!("Attr: {attr_val_as_string}");
        }

        let height_unit = read_any_attribute_to_string(&height_dataset.attr("unit")?)?;

        let heights: ndarray::Array2<f32> = height_dataset.read()?;
        log::info!("Got wasp wasp heights with {} samples", heights.len());

        let timestamps_data: ndarray::Array2<Timestamp> = timestamp_dataset.read_2d()?;
        let timestamps_raw: Vec<i64> = timestamps_data.iter().map(|t| t.time).collect();

        let mut timestamps = vec![];
        let mut first_timestamp: Option<DateTime<Utc>> = None;
        for t in timestamps_raw {
            let ts = chrono::Utc.timestamp_nanos(t).to_utc();
            if first_timestamp.is_none() {
                first_timestamp = Some(ts);
            }
            timestamps.push(
                ts.timestamp_nanos_opt()
                    .expect("timestamp as nanoseconds out of range") as f64,
            );
        }

        let timestamp_dataset_description =
            read_string_attribute(&height_dataset.attr("description")?)?;
        let timestamp_stream_descriptor_toml_str =
            read_string_attribute(&timestamp_dataset.attr("stream_descriptor")?)?;
        let Ok(timestamp_stream_descriptor): Result<StreamDescriptor, toml::de::Error> =
            toml::from_str(&timestamp_stream_descriptor_toml_str)
        else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Failed decoding 'stream_descriptor' string as TOML from stream_descriptor: {timestamp_stream_descriptor_toml_str}"
                ),
            ));
        };

        for a in timestamp_dataset.attr_names()? {
            let attr = timestamp_dataset.attr(&a)?;
            let attr_val_as_string = read_any_attribute_to_string(&attr)?;
            log::info!("Attr: {attr_val_as_string}");
        }
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

        let _timestamp_unit = read_any_attribute_to_string(&timestamp_dataset.attr("unit")?)?;

        let mut height_with_ts: Vec<[f64; 2]> = Vec::new();

        for (height, ts) in heights.iter().zip(timestamps) {
            height_with_ts.push([ts, *height as f64]);
        }

        let rawplot = RawPlot::new(
            format!("Height [{height_unit}]"),
            height_with_ts,
            ExpectedPlotRange::OneToOneHundred,
        );

        Ok(Self {
            starting_timestamp_utc: first_timestamp.expect("No first timestamp"),
            dataset_description: "Wasp200 Height".to_owned(),
            raw_plots: vec![rawplot],
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

#[cfg(test)]
mod tests {
    use super::*;
    use test_util::test_file_defs::wasp200::*;
    use testresult::TestResult;

    #[test]
    fn test_read_bifrost_current() -> TestResult {
        let wasp200 = Wasp200::from_path(wasp200())?;

        assert_eq!(wasp200.raw_plots[0].points().len(), 70971);

        Ok(())
    }
}
