use std::{
    fmt, fs,
    io::{self, BufReader},
    path::Path,
};

use chrono::{DateTime, Utc};
use entries::NavSysSpsEntry;
use header::NavSysSpsHeader;
use log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};

pub(crate) mod entries;
mod header;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct NavSysSps {
    header: NavSysSpsHeader,
    entries: Vec<NavSysSpsEntry>,
    raw_plots: Vec<RawPlot>,
}

impl NavSysSps {
    /// Read a file and attempt to deserialize a `NavSysSps` header from it
    ///
    /// Return true if a valid header was deserialized
    pub fn file_is_valid(path: &Path) -> bool {
        let Ok(file) = fs::File::open(path) else {
            return false;
        };
        let mut reader = BufReader::new(file);
        NavSysSpsHeader::from_reader(&mut reader).is_ok()
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
        let mut raw_tl1_points_roll: Vec<[f64; 2]> = Vec::new();
        let mut raw_tl2_points_roll: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_latitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_latitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_longitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_longitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_gps_time_delta_ms: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_gps_time_delta_ms: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_num_satellites: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_num_satellites: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_speed_kmh: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_speed_kmh: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_hdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_hdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_vdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_vdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_pdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_pdop: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp1_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_gp2_points_altitude: Vec<[f64; 2]> = Vec::new();
        let mut raw_mag1_points: Vec<[f64; 2]> = Vec::new();

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
                NavSysSpsEntry::MA1(e) => {
                    raw_mag1_points.push([e.timestamp_ns(), e.field_nanotesla()]);
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
                "MA1 B-field [nT]".into(),
                raw_mag1_points,
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

impl fmt::Display for NavSysSps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.header)?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

impl Plotable for NavSysSps {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.header.first_timestamp()
    }

    fn descriptive_name(&self) -> &str {
        Self::DESCRIPTIVE_NAME
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        let metadata: Vec<(String, String)> = vec![
            ("Version".into(), self.header.version().to_string()),
            (
                "NavSys Software rev.".into(),
                self.header.software_revision().to_owned(),
            ),
            (
                "TiltSensor ID".into(),
                self.header.tilt_sensor_id().to_owned(),
            ),
        ];

        Some(metadata)
    }
}

impl Parseable for NavSysSps {
    const DESCRIPTIVE_NAME: &str = "NavSys Sps";

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let mut total_bytes_read = 0;
        let (header, bytes_read) = NavSysSpsHeader::from_reader(reader)?;
        total_bytes_read += bytes_read;

        let (entries, bytes_read) = parse_to_vec(reader);
        total_bytes_read += bytes_read;

        let raw_plots = Self::build_raw_plots(&entries);

        Ok((
            Self {
                header,
                entries,
                raw_plots,
            },
            total_bytes_read,
        ))
    }

    fn is_buf_valid(buf: &[u8]) -> bool {
        let mut reader = BufReader::new(buf);
        NavSysSpsHeader::from_reader(&mut reader).is_ok()
    }
}

impl GitMetadata for NavSysSps {
    fn project_version(&self) -> Option<String> {
        Some(self.header.software_revision().to_owned())
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
    use test_util::*;

    // Example Navsys.sps data with a header and some data taken from the middle of an actual Navsys.sps log
    // these logs are very long so we don't want to add a real file to the repo test data
    const TEST_DATA: &str = "\
VER 3
MRK 2024 10 03 12 52 42 401 Navsys software rev: Build: 2.0.0.6
MRK 2024 10 03 12 52 42 417 TiltSensorID : 1459_1458
MRK 2024 10 03 12 52 42 417 CalAng 1 OffsetY: 0.4950
MRK 2024 10 03 12 52 42 417 CalAng 1 Y: 3.26488
MRK 2024 10 03 12 52 42 432 CalAng 1 OffsetX: 0.5099
MRK 2024 10 03 12 52 42 432 CalAng 1 X: 3.28745
MRK 2024 10 03 12 52 42 432 CalAng 2 OffsetY: 0.4947
MRK 2024 10 03 12 52 42 432 CalAng 2 Y: 3.34307
MRK 2024 10 03 12 52 42 432 CalAng 2 OffsetX: 0.5047
MRK 2024 10 03 12 52 42 432 CalAng 2 X: 3.30737
MA1 2024 10 03 13 33 47 035 49966.2649
TL1 2024 10 03 13 33 46 769 2.28 2.60
MA1 2024 10 03 13 33 47 055 49966.1729
MA1 2024 10 03 13 33 47 075 49966.1913
MA1 2024 10 03 13 33 47 095 49966.3017
MA1 2024 10 03 13 33 47 115 49966.2465
MA1 2024 10 03 13 33 47 135 49966.1545
HE2 2024 10 03 13 33 46 878 213.78
HE2 2024 10 03 13 33 46 878 214.00
HE2 2024 10 03 13 33 46 878 214.22
HE2 2024 10 03 13 33 46 878 214.56
HE2 2024 10 03 13 33 46 878 214.90
HE2 2024 10 03 13 33 46 878 215.15
MA1 2024 10 03 13 33 47 155 49966.3569
MA1 2024 10 03 13 33 47 175 49966.1729
MA1 2024 10 03 13 33 47 195 49966.2281
HE1 2024 10 03 13 33 46 940 99999.99
HE1 2024 10 03 13 33 46 940 201.62
HE1 2024 10 03 13 33 46 940 99999.99
HE1 2024 10 03 13 33 46 940 202.17
HE1 2024 10 03 13 33 46 940 210.32
HE1 2024 10 03 13 33 46 940 211.02
MA1 2024 10 03 13 33 47 215 49966.1729
MA1 2024 10 03 13 33 47 235 49966.1361
GP2 2024 10 03 13 33 46 971 5339.76758 910.84021 13:33:47.000 19 WGS84 130.3 0.8 1.3 1.5 193.1
GP1 2024 10 03 13 33 46 971 5339.76660 910.84003 13:33:47.000 16 WGS84 130.1 0.7 1.0 1.2 193.6
TL2 2024 10 03 13 33 46 971 2.80 2.34
MA1 2024 10 03 13 33 47 255 49966.1729
MA1 2024 10 03 13 33 47 275 49966.3509
MA1 2024 10 03 13 33 47 295 49966.1913
MA1 2024 10 03 13 33 47 315 49966.2833
MA1 2024 10 03 13 33 47 335 49966.1729
HE2 2024 10 03 13 33 47 081 99999.99
HE2 2024 10 03 13 33 47 081 99999.99
HE2 2024 10 03 13 33 47 081 99999.99
HE2 2024 10 03 13 33 47 081 99999.99
HE2 2024 10 03 13 33 47 081 99999.99
HE2 2024 10 03 13 33 47 081 99999.99
MA1 2024 10 03 13 33 47 355 49966.2281
MA1 2024 10 03 13 33 47 375 49966.2097
MA1 2024 10 03 13 33 47 395 49966.2833
MA1 2024 10 03 13 33 47 415 49966.1545
HE1 2024 10 03 13 33 47 159 99999.99
HE1 2024 10 03 13 33 47 159 211.44
HE1 2024 10 03 13 33 47 159 211.31
HE1 2024 10 03 13 33 47 159 211.55
HE1 2024 10 03 13 33 47 159 212.09
HE1 2024 10 03 13 33 47 159 212.54
HE1 2024 10 03 13 33 47 159 212.23
MA1 2024 10 03 13 33 47 435 49966.2649
MA1 2024 10 03 13 33 47 455 49966.0441
MA1 2024 10 03 13 33 47 475 49966.2465
MA1 2024 10 03 13 33 47 495 49966.2465
MA1 2024 10 03 13 33 47 515 49966.1729
MA1 2024 10 03 13 33 47 535 49966.0257
TL1 2024 10 03 13 33 47 283 0.90 5.70
MA1 2024 10 03 13 33 47 555 49966.2833
HE2 2024 10 03 13 33 47 299 216.50
HE2 2024 10 03 13 33 47 299 216.48
HE2 2024 10 03 13 33 47 299 216.64
HE2 2024 10 03 13 33 47 299 217.18
HE2 2024 10 03 13 33 47 299 217.55
HE2 2024 10 03 13 33 47 299 217.85
HE2 2024 10 03 13 33 47 299 99999.99
MA1 2024 10 03 13 33 47 575 49966.1301
MA1 2024 10 03 13 33 47 595 49966.2649
MA1 2024 10 03 13 33 47 615 49966.1545
MA1 2024 10 03 13 33 47 635 49966.3569
HE1 2024 10 03 13 33 47 377 212.36
HE1 2024 10 03 13 33 47 377 212.67
HE1 2024 10 03 13 33 47 377 212.90
HE1 2024 10 03 13 33 47 377 212.99
HE1 2024 10 03 13 33 47 377 213.29
HE1 2024 10 03 13 33 47 377 213.50
HE1 2024 10 03 13 33 47 377 213.77
MA1 2024 10 03 13 33 47 655 49966.0257
MA1 2024 10 03 13 33 47 675 49966.3201
MA1 2024 10 03 13 33 47 695 49966.0993
MA1 2024 10 03 13 33 47 715 49966.1913
MA1 2024 10 03 13 33 47 735 49966.0625
MA1 2024 10 03 13 33 47 755 49966.3201
TL2 2024 10 03 13 33 47 486 0.25 6.78
MA1 2024 10 03 13 33 47 775 49966.1361
HE2 2024 10 03 13 33 47 517 99999.99
HE2 2024 10 03 13 33 47 517 99999.99
HE2 2024 10 03 13 33 47 517 99999.99
HE2 2024 10 03 13 33 47 517 219.06
HE2 2024 10 03 13 33 47 517 219.23
HE2 2024 10 03 13 33 47 517 219.58
HE2 2024 10 03 13 33 47 517 219.96
MA1 2024 10 03 13 33 47 795 49966.1545
MA1 2024 10 03 13 33 47 815 49965.9337
MA1 2024 10 03 13 33 47 835 49966.2465
MA1 2024 10 03 13 33 47 855 49966.0993
HE1 2024 10 03 13 33 47 595 213.96
HE1 2024 10 03 13 33 47 595 214.04
HE1 2024 10 03 13 33 47 595 214.33
HE1 2024 10 03 13 33 47 595 214.53
";

    #[test]
    fn test_parse_navsys_log() -> TestResult {
        let mut cursor = io::Cursor::new(TEST_DATA);
        let (navsys, bytes_read) = NavSysSps::from_reader(&mut cursor)?;

        assert_eq!(bytes_read, TEST_DATA.len());

        assert_eq!(navsys.header.software_revision(), "Build: 2.0.0.6");
        assert_eq!(navsys.header.tilt_sensor_id(), "1459_1458");
        assert_eq!(navsys.entries.len(), 98);

        Ok(())
    }
}
