use std::{
    fmt, fs,
    io::{self, BufReader},
    path::Path,
};

use chrono::{DateTime, Utc};
use entries::NavSysSpsEntry;
use header::NavSysSpsHeader;
use plotinator_log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};

use crate::navsys::entries::gps::Gps;

pub(crate) mod entries;
pub(crate) mod header;

// Helper struct to collect GPS data during iteration
#[derive(Default)]
pub(crate) struct GpsDataCollector {
    timestamps: Vec<f64>,
    latitude: Vec<f64>,
    longitude: Vec<f64>,
    altitude: Vec<f64>,
    speed: Vec<f64>,
    time_delta: Vec<[f64; 2]>,
    satellites: Vec<[f64; 2]>,
    hdop: Vec<[f64; 2]>,
    vdop: Vec<[f64; 2]>,
    pdop: Vec<[f64; 2]>,
}

impl GpsDataCollector {
    pub(crate) fn add_entry(&mut self, e: &Gps) {
        let ts = e.timestamp_ns();

        // For GeoSpatial builder - only add if both lat and lon are valid
        if !e.latitude().is_nan() && !e.longitude().is_nan() {
            self.timestamps.push(ts);
            self.latitude.push(e.latitude_deg());
            self.longitude.push(e.longitude_deg());

            if !e.altitude_above_mean_sea().is_nan() {
                self.altitude.push(e.altitude_above_mean_sea().into());
            }
            if !e.speed_kmh().is_nan() {
                self.speed.push(e.speed_kmh().into());
            }
        }

        // Additional plots
        self.time_delta.push([ts, e.gps_time_delta_ms()]);
        self.satellites.push([ts, e.num_satellites().into()]);

        if !e.hdop().is_nan() {
            self.hdop.push([ts, e.hdop().into()]);
        }
        if !e.vdop().is_nan() {
            self.vdop.push([ts, e.vdop().into()]);
        }
        if !e.pdop().is_nan() {
            self.pdop.push([ts, e.pdop().into()]);
        }
    }

    pub(crate) fn build_plots(self, sensor_name: &str, raw_plots: &mut Vec<RawPlot>) {
        // Create GeoSpatial plot if we have valid position data
        if !self.timestamps.is_empty() {
            let mut builder = GeoSpatialDataBuilder::new(sensor_name.to_string())
                .timestamp(&self.timestamps)
                .lat(&self.latitude)
                .lon(&self.longitude);

            if !self.altitude.is_empty() {
                builder = builder.altitude(&self.altitude);
            }
            if !self.speed.is_empty() {
                builder = builder.speed(&self.speed);
            }

            if let Ok(geo_data) = builder.build() {
                raw_plots.push(RawPlot::from(geo_data));
            }
        }

        // Create additional generic plots
        if !self.time_delta.is_empty() {
            raw_plots.push(
                RawPlotCommon::new(
                    format!("Time delta [ms] ({sensor_name})"),
                    self.time_delta,
                    ExpectedPlotRange::Thousands,
                )
                .into(),
            );
        }
        if !self.satellites.is_empty() {
            raw_plots.push(
                RawPlotCommon::new(
                    format!("Satellites ({sensor_name})"),
                    self.satellites,
                    ExpectedPlotRange::OneToOneHundred,
                )
                .into(),
            );
        }
        if !self.hdop.is_empty() {
            raw_plots.push(
                RawPlotCommon::new(
                    format!("HDOP ({sensor_name})"),
                    self.hdop,
                    ExpectedPlotRange::OneToOneHundred,
                )
                .into(),
            );
        }
        if !self.vdop.is_empty() {
            raw_plots.push(
                RawPlotCommon::new(
                    format!("VDOP ({sensor_name})"),
                    self.vdop,
                    ExpectedPlotRange::OneToOneHundred,
                )
                .into(),
            );
        }
        if !self.pdop.is_empty() {
            raw_plots.push(
                RawPlotCommon::new(
                    format!("PDOP ({sensor_name})"),
                    self.pdop,
                    ExpectedPlotRange::OneToOneHundred,
                )
                .into(),
            );
        }
    }
}

// Helper struct for height sensor data
#[derive(Default)]
pub(crate) struct HeightDataCollector {
    altitude: Vec<[f64; 2]>,
    invalid: Vec<[f64; 2]>,
    invalid_count: u64,
}

impl HeightDataCollector {
    pub(crate) fn add_entry(&mut self, ts: f64, altitude: Option<f64>) {
        if let Some(alt) = altitude {
            self.altitude.push([ts, alt]);
        } else {
            self.invalid_count += 1;
            self.invalid.push([ts, self.invalid_count as f64]);
        }
    }

    pub(crate) fn build_plots(self, sensor_name: &str, raw_plots: &mut Vec<RawPlot>) {
        if !self.altitude.is_empty() {
            raw_plots.push(RawPlot::from(RawPlotCommon::new(
                format!("Altitude [M] ({sensor_name})"),
                self.altitude,
                ExpectedPlotRange::OneToOneHundred,
            )));
        }
        if !self.invalid.is_empty() {
            raw_plots.push(RawPlot::from(RawPlotCommon::new(
                format!("Invalid Count ({sensor_name})"),
                self.invalid,
                ExpectedPlotRange::Thousands,
            )));
        }
    }
}

// Helper struct for tilt sensor data
#[derive(Default)]
pub(crate) struct TiltDataCollector {
    pitch: Vec<[f64; 2]>,
    roll: Vec<[f64; 2]>,
}

impl TiltDataCollector {
    pub(crate) fn add_entry(&mut self, ts: f64, pitch: f64, roll: f64) {
        self.pitch.push([ts, pitch]);
        self.roll.push([ts, roll]);
    }

    pub(crate) fn build_plots(self, sensor_name: &str, raw_plots: &mut Vec<RawPlot>) {
        if !self.pitch.is_empty() {
            raw_plots.push(RawPlot::from(RawPlotCommon::new(
                format!("Pitch° ({sensor_name})"),
                self.pitch,
                ExpectedPlotRange::OneToOneHundred,
            )));
        }
        if !self.roll.is_empty() {
            raw_plots.push(RawPlot::from(RawPlotCommon::new(
                format!("Roll° ({sensor_name})"),
                self.roll,
                ExpectedPlotRange::OneToOneHundred,
            )));
        }
    }
}

// Helper struct for magnetometer data
#[derive(Default)]
pub(crate) struct MagDataCollector {
    values: Vec<[f64; 2]>,
}

impl MagDataCollector {
    pub(crate) fn add_entry(&mut self, ts: f64, field: f64) {
        self.values.push([ts, field]);
    }

    pub(crate) fn build_plots(self, sensor_name: &str, raw_plots: &mut Vec<RawPlot>) {
        if !self.values.is_empty() {
            raw_plots.push(
                RawPlotCommon::new(
                    format!("B-field [nT] ({sensor_name})"),
                    self.values,
                    ExpectedPlotRange::Thousands,
                )
                .into(),
            );
        }
    }
}

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
        let mut he1 = HeightDataCollector::default();
        let mut he2 = HeightDataCollector::default();
        let mut he3 = HeightDataCollector::default();
        let mut hex = HeightDataCollector::default();

        let mut tl1 = TiltDataCollector::default();
        let mut tl2 = TiltDataCollector::default();

        let mut gp1 = GpsDataCollector::default();
        let mut gp2 = GpsDataCollector::default();

        let mut ma1 = MagDataCollector::default();

        // Collect all data
        for entry in entries {
            match entry {
                NavSysSpsEntry::HE1(e) => {
                    he1.add_entry(e.timestamp_ns(), e.altitude_m());
                }
                NavSysSpsEntry::HE2(e) => {
                    he2.add_entry(e.timestamp_ns(), e.altitude_m());
                }
                NavSysSpsEntry::HE3(e) => {
                    he3.add_entry(e.timestamp_ns(), e.altitude_m());
                }
                NavSysSpsEntry::HEx(e) => {
                    hex.add_entry(e.timestamp_ns(), e.altitude_m());
                }
                NavSysSpsEntry::TL1(e) => {
                    tl1.add_entry(
                        e.timestamp_ns(),
                        e.pitch_angle_degrees(),
                        e.roll_angle_degrees(),
                    );
                }
                NavSysSpsEntry::TL2(e) => {
                    tl2.add_entry(
                        e.timestamp_ns(),
                        e.pitch_angle_degrees(),
                        e.roll_angle_degrees(),
                    );
                }
                NavSysSpsEntry::GP1(e) => {
                    gp1.add_entry(e);
                }
                NavSysSpsEntry::GP2(e) => {
                    gp2.add_entry(e);
                }
                NavSysSpsEntry::MA1(e) => {
                    ma1.add_entry(e.timestamp_ns(), e.field_nanotesla());
                }
                _ => log::error!("Ignoring unknown navsys.sps entry: {entry}"),
            }
        }

        // Build all plots
        let mut raw_plots = Vec::new();

        he1.build_plots("HE1", &mut raw_plots);
        he2.build_plots("HE2", &mut raw_plots);
        he3.build_plots("HE3", &mut raw_plots);
        hex.build_plots("HEx", &mut raw_plots);

        tl1.build_plots("TL1", &mut raw_plots);
        tl2.build_plots("TL2", &mut raw_plots);

        gp1.build_plots("GP1", &mut raw_plots);
        gp2.build_plots("GP2", &mut raw_plots);

        ma1.build_plots("MA1", &mut raw_plots);

        // Ensure no duplicate timestamps
        for plot in &mut raw_plots {
            if let RawPlot::Generic { common } = plot {
                ensure_unique_timestamps(common.points_as_mut());
            }
        }

        raw_plots
    }
}

pub(crate) fn ensure_unique_timestamps(points: &mut [[f64; 2]]) {
    let mut last_timestamp = f64::NEG_INFINITY;
    for p in points {
        if p[0] <= last_timestamp {
            let min_representable_delta = last_timestamp * f64::EPSILON;
            p[0] = last_timestamp + min_representable_delta;
        }
        last_timestamp = p[0];
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
    use plotinator_test_util::*;

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
