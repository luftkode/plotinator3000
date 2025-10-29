use std::{
    collections::HashMap,
    fmt, fs,
    io::{self, BufReader},
    path::Path,
    str::FromStr as _,
};

use chrono::{DateTime, Utc};
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};

use crate::navsys::{
    entries::tl::InclinometerEntry,
    header::{CalibrationData, TiltSensorID},
};

const LEGEND: &str = "TL";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InclinometerSps {
    tilt_sensor_id: TiltSensorID,
    calibration_data_1: CalibrationData,
    calibration_data_2: CalibrationData,
    entries: Vec<InclinometerEntry>,
    raw_plots: Vec<RawPlot>,
}

impl InclinometerSps {
    /// Read a file and attempt to deserialize a `InclinometerSps` entry from it
    ///
    /// Return true if a valid header was deserialized
    pub fn is_file_valid(path: &Path) -> bool {
        let Ok(file) = fs::File::open(path) else {
            return false;
        };
        let mut reader = BufReader::new(file);
        Self::is_reader_valid(&mut reader)
    }

    fn is_reader_valid(reader: &mut impl io::BufRead) -> bool {
        // If the sensor ID, calibration data, and 3 entries can be read successfully then it's valid
        TiltSensorID::from_reader(reader).is_ok()
            && CalibrationData::from_reader(reader).is_ok()
            && CalibrationData::from_reader(reader).is_ok()
            && InclinometerEntry::from_reader(reader).is_ok()
            && InclinometerEntry::from_reader(reader).is_ok()
            && InclinometerEntry::from_reader(reader).is_ok()
    }
}

impl fmt::Display for InclinometerSps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.entries)
    }
}

impl LogEntry for InclinometerEntry {
    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        // just a sanity check, it is definitely invalid if it is less than 10 characters
        if line.len() < 10 {
            if line.is_empty() {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "End of File"));
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Expected NavSysSps entry line but line is too short to be a NavSysSps entry. Line length={}, content={line}",
                        line.len()
                    ),
                ));
            }
        }
        let entry =
            Self::from_str(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok((entry, bytes_read))
    }

    fn timestamp_ns(&self) -> i64 {
        self.timestamp_ns()
    }
}

impl GitMetadata for InclinometerSps {
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

impl Parseable for InclinometerSps {
    const DESCRIPTIVE_NAME: &str = "InclinometerSps";

    fn from_reader(reader: &mut impl io::BufRead) -> anyhow::Result<(Self, usize)> {
        let mut bytes_read = 0;
        // Parse tilt sensor ID
        let (tilt_sensor_id, bytes_tl_id) = TiltSensorID::from_reader(reader)?;
        bytes_read += bytes_tl_id;

        // Parse both calibration data sets
        let (calibration_data_1, bytes_cal_1) = CalibrationData::from_reader(reader)?;
        let (calibration_data_2, bytes_cal_2) = CalibrationData::from_reader(reader)?;
        bytes_read += bytes_cal_1 + bytes_cal_2;

        let (entries, bytes_read_entries): (Vec<InclinometerEntry>, usize) = parse_to_vec(reader);
        bytes_read += bytes_read_entries;

        // Group entries by sensor ID
        type PitchRollTuple = (Vec<[f64; 2]>, Vec<[f64; 2]>, Vec<i64>);
        let mut sensor_groups: HashMap<u8, PitchRollTuple> = HashMap::new();
        for entry in &entries {
            let sensor_id = entry.id;
            let (pitch_points, roll_points, timestamps) = sensor_groups
                .entry(sensor_id)
                .or_insert_with(|| (Vec::new(), Vec::new(), Vec::new()));

            pitch_points.push([entry.timestamp_ns() as f64, entry.pitch_angle_degrees()]);
            roll_points.push([entry.timestamp_ns() as f64, entry.roll_angle_degrees()]);
            timestamps.push(entry.timestamp_ns());
        }

        // Create plots for each sensor
        let mut raw_plots = Vec::new();
        for (sensor_id, (pitch_points, roll_points, timestamps)) in sensor_groups {
            let mut plots = RawPlotBuilder::new(format!("{LEGEND}{sensor_id}"))
                .add_timestamp_delta(&timestamps)
                .add(pitch_points, DataType::Pitch)
                .add(roll_points, DataType::Roll)
                .build();

            raw_plots.append(&mut plots);
        }

        Ok((
            Self {
                tilt_sensor_id,
                calibration_data_1,
                calibration_data_2,
                entries,
                raw_plots,
            },
            bytes_read,
        ))
    }

    fn is_buf_valid(buf: &[u8]) -> Result<(), String> {
        let mut reader = BufReader::new(buf);
        if Self::is_reader_valid(&mut reader) {
            Ok(())
        } else {
            Err(format!(
                "Not a valid {}: line format mismatch",
                Self::DESCRIPTIVE_NAME
            ))
        }
    }
}

impl Plotable for InclinometerSps {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.entries
            .first()
            .expect("No entries in InclinometerSps, unable to get first timestamp")
            .timestamp()
    }

    fn descriptive_name(&self) -> &str {
        Self::DESCRIPTIVE_NAME
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        let metadata: Vec<(String, String)> = vec![
            (
                "TiltSensor ID".into(),
                self.tilt_sensor_id.clone().to_string(),
            ),
            (
                "#1 Calibration Data".into(),
                self.calibration_data_1.to_string(),
            ),
            (
                "#2 Calibration Data".into(),
                self.calibration_data_2.to_string(),
            ),
        ];

        Some(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::{
        njord_altimeter_wasp200_sps,
        test_file_defs::{frame_inclinometers::*, frame_magnetometer::frame_magnetometer},
    };

    #[test]
    fn test_inclinometer_sps_file_is_valid() {
        let is_valid = InclinometerSps::is_file_valid(&frame_inclinometers_sps());
        assert!(is_valid);
    }

    #[test]
    fn test_inclinometer_sps_buf_is_valid() {
        assert_eq!(
            InclinometerSps::is_buf_valid(FRAME_INCLINOMETERS_SPS_BYTES),
            Ok(())
        );
    }

    #[test]
    fn test_wasp_sps_file_is_not_valid() {
        let is_valid = InclinometerSps::is_file_valid(&njord_altimeter_wasp200_sps());
        assert!(!is_valid);
    }

    #[test]
    fn test_h5_file_is_not_valid() {
        let is_valid = InclinometerSps::is_file_valid(&frame_magnetometer());
        assert!(!is_valid);
    }

    #[test]
    fn test_deserialize_inclinometers_sps_file() {
        let mut frame_inclinometers_sps = FRAME_INCLINOMETERS_SPS_BYTES;
        let (magsps, bytes_read) =
            InclinometerSps::from_reader(&mut frame_inclinometers_sps).unwrap();
        // Windows counts newlines differently so the byte count differs
        if cfg!(target_os = "windows") {
            assert_eq!(bytes_read, 3058);
        } else {
            assert_eq!(bytes_read, 2985);
        }
        assert_eq!(magsps.entries.len(), 64);
    }
}
