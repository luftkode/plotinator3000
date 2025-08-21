use std::{
    fmt, fs,
    io::{self, BufReader},
    path::Path,
};

use chrono::{DateTime, Utc};
use plotinator_log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    mag_sps::MagSps,
    navsys::entries::{NavSysSpsEntry, tl::InclinometerEntry},
    wasp200::Wasp200Sps,
};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct NavSysSpsKitchenSink {
    first_timestamp: DateTime<Utc>,
    entries: Vec<NavSysSpsEntry>,
    raw_plots: Vec<RawPlot>,
}

impl NavSysSpsKitchenSink {
    /// Read a file and attempt to deserialize a `NavSysSps` header from it
    ///
    /// Return true if a valid header was deserialized
    pub fn file_is_valid(path: &Path) -> bool {
        let Ok(file) = fs::File::open(path) else {
            return false;
        };
        let mut reader = BufReader::new(&file);
        let valid_tilt_sensor = Self::is_reader_valid_tilt_sensor(&mut reader);

        let mut reader = BufReader::new(file);
        let valid_tilt_sensor_cal_vals = Self::is_reader_valid_tilt_sensor_cal_vals(&mut reader);

        MagSps::file_is_valid(path)
            || Wasp200Sps::file_is_valid(path)
            || valid_tilt_sensor
            || valid_tilt_sensor_cal_vals
    }

    fn is_reader_valid_tilt_sensor(reader: &mut impl io::BufRead) -> bool {
        // If 3 entries can be read successfully then it's valid
        InclinometerEntry::from_reader(reader).is_ok()
            && InclinometerEntry::from_reader(reader).is_ok()
            && InclinometerEntry::from_reader(reader).is_ok()
    }

    fn is_reader_valid_tilt_sensor_cal_vals(reader: impl io::BufRead) -> bool {
        let mut valid_lines = 0;
        for l in reader.lines() {
            let Ok(l) = l else {
                return false;
            };
            if l.starts_with("MRK") {
                valid_lines += 1;
            }

            if valid_lines == 3 {
                return true;
            }
        }
        false
    }

    #[allow(
        clippy::too_many_lines,
        reason = "There's a lot of plottable stuff in navsys sps, maybe this could be prettier, but yea..."
    )]
    fn build_raw_plots(entries: &[NavSysSpsEntry]) -> Vec<RawPlot> {
        let mut raw_he1_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_he2_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_he3_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_hex_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut he1_invalid_value_count: u64 = 0;
        let mut raw_he1_points_invalid_value: Vec<[f64; 2]> = Vec::new();
        let mut he2_invalid_value_count: u64 = 0;
        let mut raw_he2_points_invalid_value: Vec<[f64; 2]> = Vec::new();
        let mut he3_invalid_value_count: u64 = 0;
        let mut raw_he3_points_invalid_value: Vec<[f64; 2]> = Vec::new();
        let mut hex_invalid_value_count: u64 = 0;
        let mut raw_hex_points_invalid_value: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl1_points_pitch: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl2_points_pitch: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl3_points_pitch: Vec<[f64; 2]> = Vec::new();
        let mut raw_tlx_points_pitch: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl1_points_roll: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl2_points_roll: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl3_points_roll: Vec<[f64; 2]> = Vec::new();
        let mut raw_tlx_points_roll: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_latitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_latitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_latitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_latitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_longitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_longitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_longitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_longitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_gps_time_delta_ms: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_gps_time_delta_ms: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_gps_time_delta_ms: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_gps_time_delta_ms: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_num_satellites: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_num_satellites: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_num_satellites: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_num_satellites: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_speed_kmh: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_speed_kmh: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_speed_kmh: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_speed_kmh: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_hdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_hdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_hdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_hdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_vdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_vdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_vdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_vdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_pdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_pdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_pdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_pdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp3_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gpx_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_mag1_points: Vec<[f64; 2]> = Vec::new();
        let mut raw_mag2_points: Vec<[f64; 2]> = Vec::new();
        let mut raw_magx_points: Vec<[f64; 2]> = Vec::new();

        for entry in entries {
            match entry {
                NavSysSpsEntry::HE1(e) => {
                    if let Some(altitude) = e.altitude_m() {
                        raw_he1_points_altitude.push([e.timestamp_ns(), altitude]);
                    } else {
                        he1_invalid_value_count += 1;
                        raw_he1_points_invalid_value
                            .push([e.timestamp_ns(), he1_invalid_value_count as f64]);
                    }
                }
                NavSysSpsEntry::HE2(e) => {
                    if let Some(altitude) = e.altitude_m() {
                        raw_he2_points_altitude.push([e.timestamp_ns(), altitude]);
                    } else {
                        he2_invalid_value_count += 1;
                        raw_he2_points_invalid_value
                            .push([e.timestamp_ns(), he2_invalid_value_count as f64]);
                    }
                }
                NavSysSpsEntry::HE3(e) => {
                    if let Some(altitude) = e.altitude_m() {
                        raw_he3_points_altitude.push([e.timestamp_ns(), altitude]);
                    } else {
                        he3_invalid_value_count += 1;
                        raw_he3_points_invalid_value
                            .push([e.timestamp_ns(), he3_invalid_value_count as f64]);
                    }
                }
                NavSysSpsEntry::HEx(e) => {
                    if let Some(altitude) = e.altitude_m() {
                        raw_hex_points_altitude.push([e.timestamp_ns(), altitude]);
                    } else {
                        hex_invalid_value_count += 1;
                        raw_hex_points_invalid_value
                            .push([e.timestamp_ns(), hex_invalid_value_count as f64]);
                    }
                }
                NavSysSpsEntry::TL1(e) => {
                    raw_tl1_points_pitch.push([e.timestamp_ns(), e.pitch_angle_degrees()]);
                    raw_tl1_points_roll.push([e.timestamp_ns(), e.roll_angle_degrees()]);
                }
                NavSysSpsEntry::TL2(e) => {
                    raw_tl2_points_pitch.push([e.timestamp_ns(), e.pitch_angle_degrees()]);
                    raw_tl2_points_roll.push([e.timestamp_ns(), e.roll_angle_degrees()]);
                }
                NavSysSpsEntry::TL3(e) => {
                    raw_tl3_points_pitch.push([e.timestamp_ns(), e.pitch_angle_degrees()]);
                    raw_tl3_points_roll.push([e.timestamp_ns(), e.roll_angle_degrees()]);
                }
                NavSysSpsEntry::TLx(e) => {
                    raw_tlx_points_pitch.push([e.timestamp_ns(), e.pitch_angle_degrees()]);
                    raw_tlx_points_roll.push([e.timestamp_ns(), e.roll_angle_degrees()]);
                }
                NavSysSpsEntry::GP1(e) => {
                    let ts = e.timestamp_ns();
                    raw_gp1_points_latitude.push([ts, e.latitude()]);
                    raw_gp1_points_longitude.push([ts, e.longitude()]);
                    raw_gp1_points_gps_time_delta_ms.push([ts, e.gps_time_delta_ms()]);
                    raw_gp1_points_num_satellites.push([ts, e.num_satellites().into()]);
                    raw_gp1_points_speed_kmh.push([ts, e.speed_kmh().into()]);
                    raw_gp1_points_hdop.push([ts, e.hdop().into()]);
                    raw_gp1_points_vdop.push([ts, e.vdop().into()]);
                    raw_gp1_points_pdop.push([ts, e.pdop().into()]);
                    raw_gp1_points_altitude.push([ts, e.altitude_above_mean_sea().into()]);
                }
                NavSysSpsEntry::GP2(e) => {
                    let ts = e.timestamp_ns();
                    raw_gp2_points_latitude.push([ts, e.latitude()]);
                    raw_gp2_points_longitude.push([ts, e.longitude()]);
                    raw_gp2_points_gps_time_delta_ms.push([ts, e.gps_time_delta_ms()]);
                    raw_gp2_points_num_satellites.push([ts, e.num_satellites().into()]);
                    raw_gp2_points_speed_kmh.push([ts, e.speed_kmh().into()]);
                    raw_gp2_points_hdop.push([ts, e.hdop().into()]);
                    raw_gp2_points_vdop.push([ts, e.vdop().into()]);
                    raw_gp2_points_pdop.push([ts, e.pdop().into()]);
                    raw_gp2_points_altitude.push([ts, e.altitude_above_mean_sea().into()]);
                }
                NavSysSpsEntry::GP3(e) => {
                    let ts = e.timestamp_ns();
                    raw_gp3_points_latitude.push([ts, e.latitude()]);
                    raw_gp3_points_longitude.push([ts, e.longitude()]);
                    raw_gp3_points_gps_time_delta_ms.push([ts, e.gps_time_delta_ms()]);
                    raw_gp3_points_num_satellites.push([ts, e.num_satellites().into()]);
                    raw_gp3_points_speed_kmh.push([ts, e.speed_kmh().into()]);
                    raw_gp3_points_hdop.push([ts, e.hdop().into()]);
                    raw_gp3_points_vdop.push([ts, e.vdop().into()]);
                    raw_gp3_points_pdop.push([ts, e.pdop().into()]);
                    raw_gp3_points_altitude.push([ts, e.altitude_above_mean_sea().into()]);
                }
                NavSysSpsEntry::GPx(e) => {
                    let ts = e.timestamp_ns();
                    raw_gpx_points_latitude.push([ts, e.latitude()]);
                    raw_gpx_points_longitude.push([ts, e.longitude()]);
                    raw_gpx_points_gps_time_delta_ms.push([ts, e.gps_time_delta_ms()]);
                    raw_gpx_points_num_satellites.push([ts, e.num_satellites().into()]);
                    raw_gpx_points_speed_kmh.push([ts, e.speed_kmh().into()]);
                    raw_gpx_points_hdop.push([ts, e.hdop().into()]);
                    raw_gpx_points_vdop.push([ts, e.vdop().into()]);
                    raw_gpx_points_pdop.push([ts, e.pdop().into()]);
                    raw_gpx_points_altitude.push([ts, e.altitude_above_mean_sea().into()]);
                }
                NavSysSpsEntry::MA1(e) => {
                    raw_mag1_points.push([e.timestamp_ns(), e.field_nanotesla()]);
                }
                NavSysSpsEntry::MA2(e) => {
                    raw_mag2_points.push([e.timestamp_ns(), e.field_nanotesla()]);
                }
                NavSysSpsEntry::MAx(e) => {
                    raw_magx_points.push([e.timestamp_ns(), e.field_nanotesla()]);
                }
            }
        }

        let mut raw_plots = vec![
            RawPlot::new(
                "HE1 Altitude [M]".into(),
                raw_he1_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "HE2 Altitude [M]".into(),
                raw_he2_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "HE3 Altitude [M]".into(),
                raw_he3_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "HEx Altitude [M]".into(),
                raw_hex_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "HE1 Invalid Count".into(),
                raw_he1_points_invalid_value,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "HE2 Invalid Count".into(),
                raw_he2_points_invalid_value,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "HE3 Invalid Count".into(),
                raw_he3_points_invalid_value,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "HEx Invalid Count".into(),
                raw_hex_points_invalid_value,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "TL1 Pitch".into(),
                raw_tl1_points_pitch,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TL2 Pitch".into(),
                raw_tl2_points_pitch,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TL3 Pitch".into(),
                raw_tl3_points_pitch,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TLx Pitch".into(),
                raw_tlx_points_pitch,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TL1 Roll".into(),
                raw_tl1_points_roll,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TL2 Roll".into(),
                raw_tl2_points_roll,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TL3 Roll".into(),
                raw_tl3_points_roll,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "TLx Roll".into(),
                raw_tlx_points_roll,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 Latitude".into(),
                raw_gp1_points_latitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP2 Latitude".into(),
                raw_gp2_points_latitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP3 Latitude".into(),
                raw_gp3_points_latitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GPx Latitude".into(),
                raw_gpx_points_latitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP1 Longitude".into(),
                raw_gp1_points_longitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP2 Longitude".into(),
                raw_gp2_points_longitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP3 Longitude".into(),
                raw_gp3_points_longitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GPx Longitude".into(),
                raw_gpx_points_longitude,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP1 Time delta [ms]".into(),
                raw_gp1_points_gps_time_delta_ms,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP2 Time delta [ms]".into(),
                raw_gp2_points_gps_time_delta_ms,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP3 Time delta [ms]".into(),
                raw_gp3_points_gps_time_delta_ms,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GPx Time delta [ms]".into(),
                raw_gpx_points_gps_time_delta_ms,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "GP1 Satelittes".into(),
                raw_gp1_points_num_satellites,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 Satelittes".into(),
                raw_gp2_points_num_satellites,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP3 Satelittes".into(),
                raw_gp3_points_num_satellites,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GPx Satelittes".into(),
                raw_gpx_points_num_satellites,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 Speed [km/h]".into(),
                raw_gp1_points_speed_kmh,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 Speed [km/h]".into(),
                raw_gp2_points_speed_kmh,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP3 Speed [km/h]".into(),
                raw_gp3_points_speed_kmh,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GPx Speed [km/h]".into(),
                raw_gpx_points_speed_kmh,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 HDOP".into(),
                raw_gp1_points_hdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 HDOP".into(),
                raw_gp2_points_hdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP3 HDOP".into(),
                raw_gp3_points_hdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GPx HDOP".into(),
                raw_gpx_points_hdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 VDOP".into(),
                raw_gp1_points_vdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 VDOP".into(),
                raw_gp2_points_vdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP3 VDOP".into(),
                raw_gp3_points_vdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GPx VDOP".into(),
                raw_gpx_points_vdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 PDOP".into(),
                raw_gp1_points_pdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 PDOP".into(),
                raw_gp2_points_pdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP3 PDOP".into(),
                raw_gp3_points_pdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GPx PDOP".into(),
                raw_gpx_points_pdop,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP1 Altitude [m]".into(),
                raw_gp1_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP2 Altitude [m]".into(),
                raw_gp2_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GP3 Altitude [m]".into(),
                raw_gp3_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "GPx Altitude [m]".into(),
                raw_gpx_points_altitude,
                ExpectedPlotRange::OneToOneHundred,
            ),
            RawPlot::new(
                "MA1 B-field [nT]".into(),
                raw_mag1_points,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "MA2 B-field [nT]".into(),
                raw_mag2_points,
                ExpectedPlotRange::Thousands,
            ),
            RawPlot::new(
                "MAx B-field [nT]".into(),
                raw_magx_points,
                ExpectedPlotRange::Thousands,
            ),
        ];
        raw_plots.retain(|rp| {
            if rp.points().is_empty() {
                log::warn!("{} has no data", rp.name());
                false
            } else {
                true
            }
        });

        // ensure that no timestamps are identical.
        for rp in &mut raw_plots {
            // Track the last timestamp we've seen
            let mut last_timestamp = f64::NEG_INFINITY;

            for p in rp.points_as_mut() {
                if p[0] <= last_timestamp {
                    // For large nanosecond timestamps, we need to ensure the increment is enough to be represented by f64
                    // Calculate the minimum increment that will actually change the value
                    let min_representable_delta = last_timestamp * f64::EPSILON;
                    p[0] = last_timestamp + min_representable_delta;
                }

                last_timestamp = p[0];
            }
        }

        raw_plots
    }
}

impl fmt::Display for NavSysSpsKitchenSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

impl Plotable for NavSysSpsKitchenSink {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.first_timestamp
    }

    fn descriptive_name(&self) -> &str {
        Self::DESCRIPTIVE_NAME
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        None
    }
}

impl Parseable for NavSysSpsKitchenSink {
    const DESCRIPTIVE_NAME: &str = "NavSys Sps Kitchen Sink";

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let mut entries = Vec::new();
        let mut total_bytes_read = 0;

        let mut first_timestamp: Option<DateTime<Utc>> = None;

        const MAX_ERRORS_IN_A_ROW: u16 = 50;
        let mut errors_in_a_row = 0;
        loop {
            match NavSysSpsEntry::from_reader(reader) {
                Ok((entry, bytes_read)) => {
                    errors_in_a_row = 0;
                    let ts = entry.timestamp();
                    if let Some(first_ts) = &mut first_timestamp {
                        if ts < *first_ts {
                            first_timestamp = Some(ts);
                        }
                    } else {
                        first_timestamp = Some(ts);
                    }
                    entries.push(entry);
                    total_bytes_read += bytes_read;
                }
                Err(e) => {
                    errors_in_a_row += 1;
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    } else if errors_in_a_row == MAX_ERRORS_IN_A_ROW {
                        log::warn!("Max errors in a row reached, breaking");
                        break;
                    } else {
                        // Continue on error
                        log::warn!("Failed parsing log entry: {e}");
                    }
                }
            }
        }

        let raw_plots = Self::build_raw_plots(&entries);

        Ok((
            Self {
                entries,
                raw_plots,
                first_timestamp: first_timestamp
                    .expect("invalid condition, no first timestamp in dataset"),
            },
            total_bytes_read,
        ))
    }

    fn is_buf_valid(buf: &[u8]) -> bool {
        let mut reader = BufReader::new(buf);
        let is_valid_tilt_sensor = Self::is_reader_valid_tilt_sensor(&mut reader);
        let mut reader = BufReader::new(buf);
        let valid_tilt_sensor_cal_vals = Self::is_reader_valid_tilt_sensor_cal_vals(&mut reader);

        MagSps::is_buf_valid(buf)
            || Wasp200Sps::is_buf_valid(buf)
            || is_valid_tilt_sensor
            || valid_tilt_sensor_cal_vals
    }
}

impl GitMetadata for NavSysSpsKitchenSink {
    fn project_version(&self) -> Option<String> {
        None
    }

    fn git_short_sha(&self) -> Option<String> {
        None
    }

    fn git_branch(&self) -> Option<String> {
        None
    }

    fn git_repo_status(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::{
        test_file_defs::navsys_kitchen_sink::{
            NAVSYS_SPS_KITCHEN_SINK_BYTES, navsys_sps_kitchen_sink,
        },
        *,
    };

    #[test]
    fn test_is_file_valid() {
        let is_valid = NavSysSpsKitchenSink::file_is_valid(&navsys_sps_kitchen_sink());
        assert!(is_valid);
    }

    #[test]
    fn test_is_file_valid_bifrost_h5_not_valid() {
        let is_valid = NavSysSpsKitchenSink::file_is_valid(&bifrost_current());
        assert!(!is_valid);
    }

    #[test]
    fn test_parse_navsys_kitchen_sink() -> TestResult {
        let mut test_data = NAVSYS_SPS_KITCHEN_SINK_BYTES;
        let (navsys, bytes_read) = NavSysSpsKitchenSink::from_reader(&mut test_data)?;

        if cfg!(target_os = "windows") {
            assert_eq!(bytes_read, 15556);
        } else {
            assert_eq!(bytes_read, 15119);
        }
        assert_eq!(navsys.entries.len(), 437);

        Ok(())
    }
}
