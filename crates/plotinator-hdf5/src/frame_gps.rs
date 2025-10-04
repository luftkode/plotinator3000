use std::{io, path::Path};

use anyhow::{Context as _, ensure};
use chrono::{DateTime, TimeZone as _, Utc};
use hdf5::types::FixedAscii;
use ndarray::Array2;
use plotinator_log_if::{
    hdf5::SkytemHdf5,
    prelude::{ExpectedPlotRange, GeoSpatialDataBuilder, PlotLabels, Plotable, RawPlotCommon},
    rawplot::RawPlot,
};
use serde::{Deserialize, Serialize};

use crate::{
    frame_gps::iter::GpsData,
    stream_descriptor::StreamDescriptor,
    util::{self, assert_description_in_attrs, log_all_attributes, read_string_attribute},
};

mod iter;

const LEGEND_NAME: &str = "frame-GP";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameGps {
    starting_timestamp_utc: DateTime<Utc>,
    dataset_description: String,
    raw_plots: Vec<RawPlot>,
    metadata: Vec<(String, String)>,
}

impl SkytemHdf5 for FrameGps {
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let datasets = FrameGpsDatasets::open(path)?;

        let starting_timestamp_utc = datasets.first_timestamp();
        let mut raw_plots = datasets.gps_data_to_rawplots(1)?;
        let mut raw_plots2 = datasets.gps_data_to_rawplots(2)?;

        raw_plots.append(&mut raw_plots2);

        let metadata = datasets.extract_metadata()?;

        Ok(Self {
            starting_timestamp_utc,
            dataset_description: "Frame GPS".to_owned(),
            raw_plots,
            metadata,
        })
    }
}

impl Plotable for FrameGps {
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

struct FrameGpsDatasets {
    h5file: hdf5::File,
    hdop1: hdf5::Dataset,
    hdop2: hdf5::Dataset,
    vdop1: hdf5::Dataset,
    vdop2: hdf5::Dataset,
    pdop1: hdf5::Dataset,
    pdop2: hdf5::Dataset,
    gps_time1: hdf5::Dataset,
    gps_time2: hdf5::Dataset,
    position1: hdf5::Dataset,
    position2: hdf5::Dataset,
    mode1: hdf5::Dataset,
    mode2: hdf5::Dataset,
    speed1: hdf5::Dataset,
    speed2: hdf5::Dataset,
    sats1: hdf5::Dataset,
    sats2: hdf5::Dataset,
    timestamp1: hdf5::Dataset,
    timestamp2: hdf5::Dataset,
}

impl FrameGpsDatasets {
    const GPS_TIME: &str = "gps-time-";
    const HDOP: &str = "hdop-";
    const PDOP: &str = "pdop-";
    const VDOP: &str = "vdop-";
    const MODE: &str = "mode-";
    const POSITION: &str = "position-";
    const SATELLITES: &str = "satellites-";
    const SPEED: &str = "speed-";
    const TIMESTAMP: &str = "timestamp-";

    fn open(h5file: impl AsRef<Path>) -> io::Result<Self> {
        let h5file = hdf5::File::open(h5file)?;

        let hdop1 = util::open_dataset(&h5file, &format!("{}1", Self::HDOP), 2)?;
        let hdop2 = util::open_dataset(&h5file, &format!("{}2", Self::HDOP), 2)?;
        let pdop1 = util::open_dataset(&h5file, &format!("{}1", Self::PDOP), 2)?;
        let pdop2 = util::open_dataset(&h5file, &format!("{}2", Self::PDOP), 2)?;
        let vdop1 = util::open_dataset(&h5file, &format!("{}1", Self::VDOP), 2)?;
        let vdop2 = util::open_dataset(&h5file, &format!("{}2", Self::VDOP), 2)?;

        let gps_time1 = util::open_dataset(&h5file, &format!("{}1", Self::GPS_TIME), 2)?;
        let gps_time2 = util::open_dataset(&h5file, &format!("{}2", Self::GPS_TIME), 2)?;

        let mode1 = util::open_dataset(&h5file, &format!("{}1", Self::MODE), 2)?;
        let mode2 = util::open_dataset(&h5file, &format!("{}2", Self::MODE), 2)?;

        let sats1 = util::open_dataset(&h5file, &format!("{}1", Self::SATELLITES), 2)?;
        let sats2 = util::open_dataset(&h5file, &format!("{}2", Self::SATELLITES), 2)?;

        let speed1 = util::open_dataset(&h5file, &format!("{}1", Self::SPEED), 2)?;
        let speed2 = util::open_dataset(&h5file, &format!("{}2", Self::SPEED), 2)?;

        let timestamp1 = util::open_dataset(&h5file, &format!("{}1", Self::TIMESTAMP), 2)?;
        let timestamp2 = util::open_dataset(&h5file, &format!("{}2", Self::TIMESTAMP), 2)?;

        let position1 = util::open_dataset(&h5file, &format!("{}1", Self::POSITION), 2)?;
        let position2 = util::open_dataset(&h5file, &format!("{}2", Self::POSITION), 2)?;

        Ok(Self {
            h5file,
            hdop1,
            hdop2,
            vdop1,
            vdop2,
            pdop1,
            pdop2,
            gps_time1,
            gps_time2,
            position1,
            position2,
            mode1,
            mode2,
            speed1,
            speed2,
            sats1,
            sats2,
            timestamp1,
            timestamp2,
        })
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        let get_first = |id| -> Option<DateTime<Utc>> {
            self.timestamp_dataset(id)
                .ok()
                .and_then(|ds| ds.first().copied())
                .map(|nanos| Utc.timestamp_nanos(nanos))
        };

        match (get_first(1), get_first(2)) {
            (Some(t1), Some(t2)) => Some(t1.min(t2)),
            (Some(t1), None) => Some(t1),
            (None, Some(t2)) => Some(t2),
            (None, None) => None,
        }
        .unwrap_or_default()
    }

    fn extract_metadata(&self) -> io::Result<Vec<(String, String)>> {
        assert_description_in_attrs(&self.hdop1)?;
        let mut metadata = Vec::new();

        let add_meta = |metadata: &mut Vec<(String, String)>,
                        label: &str,
                        ds: &hdf5::Dataset|
         -> io::Result<()> {
            log_all_attributes(ds);
            if let Ok(attr) = ds.attr("description") {
                let ds_descr = read_string_attribute(&attr)?;
                metadata.push((format!("{label} Description"), ds_descr));
            }
            if let Ok(stream_descr) = StreamDescriptor::try_from(ds) {
                metadata.extend_from_slice(&stream_descr.to_metadata());
            }
            Ok(())
        };

        add_meta(&mut metadata, "Dataset-1 HDOP", &self.hdop1)?;
        add_meta(&mut metadata, "Dataset-1 VDOP", &self.vdop1)?;
        add_meta(&mut metadata, "Dataset-1 PDOP", &self.pdop1)?;
        add_meta(&mut metadata, "Dataset-1 GPS Time", &self.gps_time1)?;
        add_meta(&mut metadata, "Dataset-1 Mode", &self.mode1)?;
        add_meta(&mut metadata, "Dataset-1 Position", &self.position1)?;
        add_meta(&mut metadata, "Dataset-1 Satellites", &self.sats1)?;
        add_meta(&mut metadata, "Dataset-1 Speed", &self.speed1)?;
        add_meta(&mut metadata, "Dataset-1 Timestamp", &self.timestamp1)?;

        add_meta(&mut metadata, "Dataset-2 HDOP", &self.hdop2)?;
        add_meta(&mut metadata, "Dataset-2 VDOP", &self.vdop2)?;
        add_meta(&mut metadata, "Dataset-2 PDOP", &self.pdop2)?;
        add_meta(&mut metadata, "Dataset-2 GPS Time", &self.gps_time2)?;
        add_meta(&mut metadata, "Dataset-2 Mode", &self.mode2)?;
        add_meta(&mut metadata, "Dataset-2 Position", &self.position2)?;
        add_meta(&mut metadata, "Dataset-2 Satellites", &self.sats2)?;
        add_meta(&mut metadata, "Dataset-2 Speed", &self.speed2)?;
        add_meta(&mut metadata, "Dataset-2 Timestamp", &self.timestamp2)?;

        Ok(metadata)
    }

    #[allow(clippy::too_many_lines, reason = "long but simple")]
    fn gps_data_to_rawplots(&self, id: u8) -> anyhow::Result<Vec<RawPlot>> {
        let gps_data = self.gps_data(id)?;
        let data_len = self.len(id)?;
        let mut timestamps = Vec::with_capacity(data_len);
        let mut hdop = Vec::with_capacity(data_len);
        let mut pdop = Vec::with_capacity(data_len);
        let mut vdop = Vec::with_capacity(data_len);
        let mut gps_time_offset = Vec::with_capacity(data_len);
        let mut mode = Vec::with_capacity(data_len);
        let mut lat: Vec<f64> = Vec::with_capacity(data_len);
        let mut lon: Vec<f64> = Vec::with_capacity(data_len);
        let mut alt: Vec<f64> = Vec::with_capacity(data_len);
        let mut speed: Vec<f64> = Vec::with_capacity(data_len);
        let mut sats = Vec::with_capacity(data_len);
        let mut alt_nan_count = Vec::with_capacity(data_len);
        let mut alt_nan_bool = Vec::with_capacity(data_len);
        let mut alt_nan_cnt: u64 = 0;

        for entry in gps_data.iter() {
            let ts = *entry.timestamp as f64;
            timestamps.push(ts);

            let hdop_sample = *entry.hdop;
            if !hdop_sample.is_nan() {
                hdop.push([ts, *entry.hdop]);
            }
            let pdop_sample = *entry.pdop;
            if !pdop_sample.is_nan() {
                pdop.push([ts, *entry.pdop]);
            }
            let vdop_sample = *entry.vdop;
            if !vdop_sample.is_nan() {
                vdop.push([ts, *entry.vdop]);
            }
            mode.push([ts, *entry.mode as f64]);
            let speed_sample = *entry.speed;
            if !speed_sample.is_nan() {
                speed.push((*entry.speed).into());
            }
            sats.push([ts, *entry.satellites as f64]);

            if entry.position.len() >= 3 {
                let lat_sample = entry.position[0];
                let lon_sample = entry.position[1];
                let alt_sample = entry.position[2];
                if !lat_sample.is_nan() {
                    lat.push(lat_sample);
                }
                if !lon_sample.is_nan() {
                    lon.push(lon_sample);
                }
                if alt_sample.is_nan() {
                    alt_nan_cnt += 1;
                    alt_nan_bool.push([ts, 1.0]);
                } else {
                    alt_nan_bool.push([ts, 0.0]);
                    alt.push(alt_sample);
                }
                alt_nan_count.push([ts, alt_nan_cnt as f64]);
            } else {
                log::error!(
                    "Expected entry position of length 3 or greater, got: {}",
                    entry.position
                );
            }

            let gps_time = entry.gps_time.as_str();
            if let Ok(gps_dt) = DateTime::parse_from_rfc3339(gps_time) {
                let sys_dt = Utc.timestamp_nanos(*entry.timestamp);
                let offset = sys_dt.signed_duration_since(gps_dt).num_milliseconds() as f64;
                gps_time_offset.push([ts, offset]);
            } else {
                log::warn!("Invalid GPS time offset: {gps_time}");
            };
        }

        let geo_data: RawPlot = GeoSpatialDataBuilder::new(format!("({LEGEND_NAME}{id})"))
            .timestamp(&timestamps)
            .lat(&lat)
            .lon(&lon)
            .altitude(&alt)
            .speed(&speed)
            .build()?
            .into();

        let raw_plots = vec![
            RawPlotCommon::new(
                format!("HDOP ({LEGEND_NAME}{id})"),
                hdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlotCommon::new(
                format!("PDOP ({LEGEND_NAME}{id})"),
                pdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlotCommon::new(
                format!("VDOP ({LEGEND_NAME}{id})"),
                vdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlotCommon::new(
                format!("Time Offset [ms] ({LEGEND_NAME}{id})"),
                gps_time_offset,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlotCommon::new(
                format!("Mode ({LEGEND_NAME}{id})"),
                mode,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlotCommon::new(
                format!("Altitude-NaN-cnt ({LEGEND_NAME}{id})"),
                alt_nan_count,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlotCommon::new(
                format!("Altitude-NaN ({LEGEND_NAME}{id})"),
                alt_nan_bool,
                ExpectedPlotRange::Percentage,
            ),
            RawPlotCommon::new(
                format!("Satellites ({LEGEND_NAME}{id})"),
                sats,
                ExpectedPlotRange::OneToOneHundred,
            ),
        ];

        let mut plots = vec![geo_data];
        for rp in raw_plots {
            if rp.points().is_empty() {
                log::debug!("{} has no data", rp.name());
            } else {
                plots.push(rp.into());
            }
        }

        Ok(plots)
    }

    fn timestamp_dataset(&self, id: u8) -> io::Result<Array2<i64>> {
        let ds = self.h5file.dataset(&format!("{}{id}", Self::TIMESTAMP))?;
        let time = match ds.read_2d() {
            Ok(time) => time,
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed reading timestamp dataset: {e}"),
            ))?,
        };
        Ok(time)
    }

    fn gps_time_dataset(&self, id: u8) -> io::Result<hdf5::Dataset> {
        match self.h5file.dataset(&format!("{}{id}", Self::GPS_TIME)) {
            Ok(d) => Ok(d),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed reading gps time dataset to check size: {e}"),
            )),
        }
    }

    fn gps_time_dataset_array(&self, id: u8) -> io::Result<Array2<FixedAscii<30>>> {
        let ds = self.gps_time_dataset(id)?;
        let gps_time: Array2<FixedAscii<30>> = match ds.read_2d() {
            Ok(gps_time) => gps_time,
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed reading GPS time dataset: {e}"),
            ))?,
        };

        Ok(gps_time)
    }

    fn len(&self, id: u8) -> io::Result<usize> {
        let gps_time = self.gps_time_dataset(id)?;
        Ok(gps_time.size())
    }

    fn gps_data(&self, id: u8) -> io::Result<GpsData> {
        match self.gps_data_inner(id) {
            Ok(data) => Ok(data),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed reading gps dataset: {e}"),
            )),
        }
    }

    #[allow(clippy::too_many_lines, reason = "Simple code, but a lot")]
    fn gps_data_inner(&self, id: u8) -> anyhow::Result<GpsData> {
        let (gps_time, hdop, pdop, vdop, mode, satellites, speed, timestamp, position) = match id {
            1 => {
                let gps_time = self.gps_time_dataset_array(1)?;
                let hdop = self
                    .hdop1
                    .read_2d()
                    .context("failed reading hdop1 dataset")?;
                let pdop = self
                    .pdop1
                    .read_2d()
                    .context("failed reading pdop1 dataset")?;
                let vdop = self
                    .vdop1
                    .read_2d()
                    .context("failed reading vdop1 dataset")?;
                let mode = self
                    .mode1
                    .read_2d()
                    .context("failed reading mode1 dataset")?;
                let satellites = self
                    .sats1
                    .read_2d()
                    .context("failed reading satellites1 dataset")?;
                let speed = self
                    .speed1
                    .read_2d()
                    .context("failed reading speed1 dataset")?;
                let timestamp = self
                    .timestamp1
                    .read_2d()
                    .context("failed reading timestamp1 dataset")?;
                let position = self
                    .position1
                    .read_2d()
                    .context("failed reading position1 dataset")?;
                (
                    gps_time, hdop, pdop, vdop, mode, satellites, speed, timestamp, position,
                )
            }
            2 => {
                let gps_time = self.gps_time_dataset_array(2)?;
                let hdop = self
                    .hdop2
                    .read_2d()
                    .context("failed reading hdop2 dataset")?;
                let pdop = self
                    .pdop2
                    .read_2d()
                    .context("failed reading pdop2 dataset")?;
                let vdop = self
                    .vdop2
                    .read_2d()
                    .context("failed reading vdop2 dataset")?;
                let mode = self
                    .mode2
                    .read_2d()
                    .context("failed reading mode2 dataset")?;
                let satellites = self
                    .sats2
                    .read_2d()
                    .context("failed reading satellites2 dataset")?;
                let speed = self
                    .speed2
                    .read_2d()
                    .context("failed reading speed2 dataset")?;
                let timestamp = self
                    .timestamp2
                    .read_2d()
                    .context("failed reading timestamp2 dataset")?;
                let position = self
                    .position2
                    .read_2d()
                    .context("failed reading position2 dataset")?;
                (
                    gps_time, hdop, pdop, vdop, mode, satellites, speed, timestamp, position,
                )
            }
            _ => anyhow::bail!("invalid GPS dataset id: {id}"),
        };

        let len = gps_time.len();
        ensure!(hdop.len() == len, "HDOP dataset has mismatched length");
        ensure!(pdop.len() == len, "PDOP dataset has mismatched length");
        ensure!(vdop.len() == len, "VDOP dataset has mismatched length");
        ensure!(mode.len() == len, "Mode dataset has mismatched length");
        ensure!(
            satellites.len() == len,
            "Satellites dataset has mismatched length"
        );
        ensure!(speed.len() == len, "Speed dataset has mismatched length");
        ensure!(
            timestamp.len() == len,
            "Timestamp dataset has mismatched length"
        );
        ensure!(
            position.shape()[0] == len,
            "Position dataset has mismatched length"
        );

        Ok(GpsData {
            gps_time,
            hdop,
            pdop,
            vdop,
            mode,
            position,
            satellites,
            speed,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_gps::iter::GpsEntry;
    use plotinator_test_util::test_file_defs::frame_gps::frame_gps;
    use testresult::TestResult;

    #[test]
    fn test_read_frame_gps() -> TestResult {
        let datasets = FrameGpsDatasets::open(frame_gps())?;

        let gps_data = datasets.gps_data_inner(2)?;
        let gps_entries: Vec<GpsEntry> = gps_data.iter().collect();
        for gps in &gps_entries {
            eprintln!("{gps:?}");
        }

        insta::assert_debug_snapshot!(gps_entries);

        Ok(())
    }
}
