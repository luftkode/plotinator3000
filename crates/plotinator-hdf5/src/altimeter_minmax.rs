use crate::util::read_any_attribute_to_string;
use chrono::{DateTime, NaiveDateTime, Utc};
use hdf5::types::VarLenUnicode;
use ndarray::Array1;
use plotinator_log_if::{
    hdf5::SkytemHdf5,
    prelude::{GeoSpatialDataBuilder, Plotable},
    rawplot::RawPlot,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

const ALTITUDE_VALID_RANGE: (f64, f64) = (0.0, 500.0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltimeterMinMax {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl AltimeterMinMax {
    fn process_sensor(
        h5: &hdf5::File,
        sensor_id: u8,
        sensor_type: &str,
        raw_plots: &mut Vec<RawPlot>,
    ) -> anyhow::Result<()> {
        let times: Vec<u64> = h5.dataset(&format!("timestamp_{sensor_id}"))?.read_raw()?;

        for (suffix, dataset_prefix) in [("min", "height_min"), ("max", "height_max")] {
            let heights: Array1<f32> = h5
                .dataset(&format!("{dataset_prefix}_{sensor_id}"))?
                .read_1d()?;
            let heights: Vec<f64> = heights.into_iter().map(|h| h.into()).collect();
            let legend_name = format!("{sensor_type}-{suffix}-{sensor_id}");

            if let Some(plot) = GeoSpatialDataBuilder::new(legend_name)
                .timestamp(&times)
                .altitude_from_laser(heights)
                .altitude_valid_range(ALTITUDE_VALID_RANGE)
                .build_into_rawplot()?
            {
                raw_plots.push(plot);
            }
        }

        Ok(())
    }
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

        let metadata: Vec<(String, String)> = h5
            .attr_names()?
            .into_iter()
            .filter_map(|attr_name| {
                let attr = h5.attr(&attr_name).ok()?;
                let attr_val = read_any_attribute_to_string(&attr).ok()?;
                Some((attr_name, attr_val))
            })
            .collect();

        let mut raw_plots = vec![];
        for sensor_id in 1..=sensor_count {
            Self::process_sensor(&h5, sensor_id, &sensor_type, &mut raw_plots)?;
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
