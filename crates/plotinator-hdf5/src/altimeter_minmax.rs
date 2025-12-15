use std::path::Path;

use chrono::{DateTime, NaiveDateTime, Utc};
use hdf5::types::VarLenUnicode;
use ndarray::Array1;
use plotinator_log_if::{
    hdf5::SkytemHdf5,
    prelude::{GeoSpatialDataBuilder, Plotable},
    rawplot::RawPlot,
};
use serde::{Deserialize, Serialize};

use crate::util::read_any_attribute_to_string;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltimeterMinMax {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl SkytemHdf5 for AltimeterMinMax {
    const DESCRIPTIVE_NAME: &str = "Generic Altimeter Min/Max";

    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let h5 = hdf5::File::open(path)?;
        let sensor_count = h5.attr("sensor_count")?.read_scalar::<u8>()?;
        let sensor_type = h5
            .attr("sensor_type")?
            .read_scalar::<VarLenUnicode>()?
            .to_string();
        let starting_timestamp = h5
            .attr("timestamp")?
            .read_scalar::<VarLenUnicode>()?
            .to_string();
        let starting_timestamp_utc: DateTime<Utc> =
            NaiveDateTime::parse_from_str(&starting_timestamp, "%Y%m%d_%H%M%S")?.and_utc();

        let attr_names = h5.attr_names()?;
        let mut metadata: Vec<(String, String)> = Vec::with_capacity(attr_names.len());
        for attr_name in attr_names {
            let attr = h5.attr(&attr_name)?;
            let attr_val = read_any_attribute_to_string(&attr)?;
            metadata.push((attr_name, attr_val));
        }

        let mut raw_plots = vec![];
        for sensor_id in 1..=sensor_count {
            let height_min_ds_name = format!("height_min_{sensor_id}");
            let height_max_ds_name = format!("height_max_{sensor_id}");
            let timestamp_ds_name = format!("timestamp_{sensor_id}");
            let heights_min: Array1<f32> = h5.dataset(&height_min_ds_name)?.read_1d()?;
            let heights_max: Array1<f32> = h5.dataset(&height_max_ds_name)?.read_1d()?;
            let heights_min: Vec<f64> = heights_min.into_iter().map(|h| h.into()).collect();
            let heights_max: Vec<f64> = heights_max.into_iter().map(|h| h.into()).collect();
            let times: Vec<u64> = h5.dataset(&timestamp_ds_name)?.read_raw()?;
            let legend_name_min = format!("{sensor_type}-min-{sensor_id}");
            let legend_name_max = format!("{sensor_type}-max-{sensor_id}");
            if let Some(dataseries) = GeoSpatialDataBuilder::new(legend_name_min)
                .timestamp(&times)
                .altitude_from_laser(heights_min)
                .altitude_valid_range((0.0, 500.)) // Safe to say it's invalid if it's above 500m
                .build_into_rawplot()?
            {
                raw_plots.push(dataseries);
            }
            if let Some(dataseries) = GeoSpatialDataBuilder::new(legend_name_max)
                .timestamp(&times)
                .altitude_from_laser(heights_max)
                .altitude_valid_range((0.0, 500.)) // Safe to say it's invalid if it's above 500m
                .build_into_rawplot()?
            {
                raw_plots.push(dataseries);
            }
        }

        Ok(Self {
            starting_timestamp_utc,
            dataset_description: "Generic Altimeter(s) min/max".to_owned(),
            raw_plots,
            metadata,
        })
    }
}

impl Plotable for AltimeterMinMax {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.starting_timestamp_utc
    }

    fn descriptive_name(&self) -> &str {
        Self::DESCRIPTIVE_NAME
    }

    fn labels(&self) -> Option<&[plotinator_log_if::prelude::PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        Some(self.metadata.clone())
    }
}
