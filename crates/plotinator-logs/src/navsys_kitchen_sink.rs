use std::{
    fmt, fs,
    io::{self, BufReader},
    path::Path,
    str::FromStr,
};

use chrono::{DateTime, Utc};
use plotinator_log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    mag_sps::MagSps,
    navsys::{
        GpsDataCollector, HeightDataCollector, MagDataCollector, TiltDataCollector,
        ensure_unique_timestamps,
        entries::{NavSysSpsEntry, gps::Gps, tl::InclinometerEntry},
    },
    wasp200::Wasp200Sps,
};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct NavSysSpsKitchenSink {
    first_timestamp: DateTime<Utc>,
    entries: Vec<NavSysSpsEntry>,
    raw_plots: Vec<RawPlot>,
}

impl NavSysSpsKitchenSink {
    /// Read a file and attempt to deserialize any valid `NavSysSps` entries from it
    pub fn file_is_valid(path: &Path) -> bool {
        let Ok(file) = fs::File::open(path) else {
            return false;
        };
        let mut reader = BufReader::new(&file);
        let valid_tilt_sensor = Self::is_reader_valid_tilt_sensor(&mut reader);

        let mut reader = BufReader::new(&file);
        let valid_tilt_sensor_cal_vals = Self::is_reader_valid_tilt_sensor_cal_vals(&mut reader);

        let mut reader = BufReader::new(&file);
        let valid_gps = Self::is_reader_valid_gps(&mut reader);

        MagSps::file_is_valid(path)
            || Wasp200Sps::file_is_valid(path)
            || valid_tilt_sensor
            || valid_tilt_sensor_cal_vals
            || valid_gps
    }

    fn is_reader_valid_gps(reader: &mut impl io::BufRead) -> bool {
        // If 3 entries can be read successfully then it's valid
        let mut line = String::new();
        for _ in 0..=3 {
            line.clear();
            let Ok(_bytes_read) = reader.read_line(&mut line) else {
                return false;
            };
            if let Err(e) = Gps::from_str(&line) {
                log::debug!("'{line}' is not a valid NavSys GPS line: {e}");
                return false;
            }
        }
        true
    }

    fn is_reader_valid_tilt_sensor(reader: &mut impl io::BufRead) -> bool {
        // If 3 entries can be read successfully then it's valid
        for _ in 0..=3 {
            if let Err(e) = InclinometerEntry::from_reader(reader) {
                log::debug!("Not a valid NavSys TL line: {e}");
                return false;
            }
        }
        true
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
        let mut he1 = HeightDataCollector::default();
        let mut he2 = HeightDataCollector::default();
        let mut he3 = HeightDataCollector::default();
        let mut hex = HeightDataCollector::default();

        let mut tl1 = TiltDataCollector::default();
        let mut tl2 = TiltDataCollector::default();
        let mut tl3 = TiltDataCollector::default();
        let mut tlx = TiltDataCollector::default();

        let mut gp1 = GpsDataCollector::default();
        let mut gp2 = GpsDataCollector::default();
        let mut gp3 = GpsDataCollector::default();
        let mut gpx = GpsDataCollector::default();

        let mut ma1 = MagDataCollector::default();
        let mut ma2 = MagDataCollector::default();
        let mut max = MagDataCollector::default();
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
                NavSysSpsEntry::TL3(e) => {
                    tl3.add_entry(
                        e.timestamp_ns(),
                        e.pitch_angle_degrees(),
                        e.roll_angle_degrees(),
                    );
                }
                NavSysSpsEntry::TLx(e) => {
                    tlx.add_entry(
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
                NavSysSpsEntry::GP3(e) => {
                    gp3.add_entry(e);
                }
                NavSysSpsEntry::GPx(e) => {
                    gpx.add_entry(e);
                }
                NavSysSpsEntry::MA1(e) => {
                    ma1.add_entry(e.timestamp_ns(), e.field_nanotesla());
                }
                NavSysSpsEntry::MA2(e) => {
                    ma2.add_entry(e.timestamp_ns(), e.field_nanotesla());
                }
                NavSysSpsEntry::MAx(e) => {
                    max.add_entry(e.timestamp_ns(), e.field_nanotesla());
                }
            }
        }
        let mut raw_plots = Vec::new();

        he1.build_plots("HE1", &mut raw_plots);
        he2.build_plots("HE2", &mut raw_plots);
        he3.build_plots("HE3", &mut raw_plots);
        hex.build_plots("HEx", &mut raw_plots);

        tl1.build_plots("TL1", &mut raw_plots);
        tl2.build_plots("TL2", &mut raw_plots);
        tl3.build_plots("TL3", &mut raw_plots);
        tlx.build_plots("TLx", &mut raw_plots);

        gp1.build_plots("GP1", &mut raw_plots);
        gp2.build_plots("GP2", &mut raw_plots);
        gp3.build_plots("GP3", &mut raw_plots);
        gpx.build_plots("GPx", &mut raw_plots);

        ma1.build_plots("MA1", &mut raw_plots);
        ma2.build_plots("MA2", &mut raw_plots);
        max.build_plots("MAx", &mut raw_plots);

        for plot in &mut raw_plots {
            if let RawPlot::Generic { common } = plot {
                ensure_unique_timestamps(common.points_as_mut());
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
        let mut reader = BufReader::new(buf);
        let valid_gps = Self::is_reader_valid_gps(&mut reader);

        MagSps::is_buf_valid(buf)
            || Wasp200Sps::is_buf_valid(buf)
            || is_valid_tilt_sensor
            || valid_tilt_sensor_cal_vals
            || valid_gps
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
