use crate::util::read_any_attribute_to_string;
use chrono::{DateTime, NaiveDateTime, Utc};
use hdf5::types::{VarLenAscii, VarLenUnicode};
use ndarray::Array1;
use plotinator_log_if::{
    hdf5::SkytemHdf5,
    prelude::{DataType, GeoSpatialDataBuilder, Plotable},
    rawplot::{RawPlot, RawPlotBuilder},
};
use plotinator_ui_util::ExpectedPlotRange;
use serde::{Deserialize, Serialize};
use std::path::Path;

fn zip_ts<T>(timestamps: &[T], values: &[f64]) -> Vec<[f64; 2]>
where
    T: Copy + Into<u64>,
{
    timestamps
        .iter()
        .zip(values.iter())
        .map(|(t, v)| [(*t).into() as f64, *v])
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gps {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl SkytemHdf5 for Gps {
    const DESCRIPTIVE_NAME: &str = "Generic GPS";

    #[allow(
        clippy::too_many_lines,
        reason = "just a bit too many, if this needs changing, then a refactor is in order"
    )]
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

        // Read all attributes for metadata
        let attr_names = h5.attr_names()?;
        let mut metadata: Vec<(String, String)> = Vec::with_capacity(attr_names.len());
        for attr_name in attr_names {
            let attr = h5.attr(&attr_name)?;
            let attr_val = read_any_attribute_to_string(&attr)?;
            metadata.push((attr_name, attr_val));
        }

        let mut raw_plots = vec![];

        for sensor_id in 1..=sensor_count {
            let timestamp_ds_name = format!("timestamp_{sensor_id}");
            let times: Vec<u64> = h5.dataset(&timestamp_ds_name)?.read_raw()?;

            // Read latitude
            let latitude_ds_name = format!("latitude_{sensor_id}");
            let latitude: Array1<f64> = h5.dataset(&latitude_ds_name)?.read_1d()?;
            let latitude: Vec<f64> = latitude.into_iter().collect();

            // Read longitude
            let longitude_ds_name = format!("longitude_{sensor_id}");
            let longitude: Array1<f64> = h5.dataset(&longitude_ds_name)?.read_1d()?;
            let longitude: Vec<f64> = longitude.into_iter().collect();

            // Read altitude
            let altitude_ds_name = format!("altitude_{sensor_id}");
            let altitude: Array1<f32> = h5.dataset(&altitude_ds_name)?.read_1d()?;
            let altitude: Vec<f64> = altitude.into_iter().map(|a| a.into()).collect();

            let speed: Array1<f32> = h5
                .dataset(&format!("speed_{sensor_id}"))?
                .read_1d::<f32>()?;
            let speed: Vec<f64> = speed.into_iter().map(|s| s.into()).collect();

            let legend_name = format!("{sensor_type}-{sensor_id}");

            if let Some(dataseries) = GeoSpatialDataBuilder::new(legend_name.clone())
                .timestamp(&times)
                .lat(&latitude)
                .lon(&longitude)
                .altitude_from_gnss(altitude)
                .speed(&speed)
                .build_into_rawplot()?
            {
                raw_plots.push(dataseries);
            }

            let hdop: Array1<f32> = h5.dataset(&format!("hdop_{sensor_id}"))?.read_1d()?;
            let hdop: Vec<f64> = hdop.into_iter().map(|e| e.into()).collect();
            let vdop: Array1<f32> = h5.dataset(&format!("vdop_{sensor_id}"))?.read_1d()?;
            let vdop: Vec<f64> = vdop.into_iter().map(|e| e.into()).collect();
            let pdop: Array1<f32> = h5.dataset(&format!("pdop_{sensor_id}"))?.read_1d()?;
            let pdop: Vec<f64> = pdop.into_iter().map(|e| e.into()).collect();
            let satellites: Array1<u8> =
                h5.dataset(&format!("satellites_{sensor_id}"))?.read_1d()?;
            let satellites: Vec<f64> = satellites.into_iter().map(|e| e.into()).collect();
            let gps_fix: Array1<u8> = h5.dataset(&format!("gps_fix_{sensor_id}"))?.read_1d()?;
            let gps_fix: Vec<f64> = gps_fix.into_iter().map(|e| e.into()).collect();
            let gps_time: Array1<VarLenAscii> =
                h5.dataset(&format!("gps_time_{sensor_id}"))?.read_1d()?;
            let gps_time: Vec<i64> = gps_time
                .into_iter()
                .flat_map(|e| {
                    let s = e.to_string();
                    s.parse::<DateTime<Utc>>().map(|t| t.timestamp_nanos_opt())
                })
                .flatten()
                .collect();

            let mut builder = RawPlotBuilder::new(legend_name.clone())
                .add_timestamp_delta(&times)
                .add(
                    zip_ts(&times, &hdop),
                    DataType::other_unitless("HDOP", ExpectedPlotRange::Hundreds, true),
                )
                .add(
                    zip_ts(&times, &vdop),
                    DataType::other_unitless("VDOP", ExpectedPlotRange::Hundreds, true),
                )
                .add(
                    zip_ts(&times, &pdop),
                    DataType::other_unitless("PDOP", ExpectedPlotRange::Hundreds, true),
                )
                .add(
                    zip_ts(&times, &satellites),
                    DataType::Other {
                        name: "Satellites".into(),
                        unit: None,
                        plot_range: ExpectedPlotRange::Hundreds,
                        default_hidden: true,
                    },
                )
                .add(zip_ts(&times, &gps_fix), DataType::bool("GPS Fix", true));

            // GPS time delta (GPS time vs system timestamp)
            if gps_time.len() == times.len() {
                let gps_delta: Vec<[f64; 2]> = times
                    .iter()
                    .zip(gps_time.iter())
                    .map(|(sys, gps)| [*sys as f64, (*gps - *sys as i64) as f64 * 0.000_001])
                    .collect();

                builder = builder.add(
                    gps_delta,
                    DataType::TimeDelta {
                        name: "GPS vs System".into(),
                        unit: "ms".into(),
                    },
                );
            }

            raw_plots.extend(builder.build());
        }

        Ok(Self {
            starting_timestamp_utc,
            dataset_description: "Generic GPS Receiver(s)".to_owned(),
            raw_plots,
            metadata,
        })
    }
}

impl Plotable for Gps {
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
