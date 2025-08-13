use std::{
    fmt, fs,
    io::{self, BufReader},
    path::Path,
    str::FromStr as _,
};

use chrono::{DateTime, Utc};
use plotinator_log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};

use crate::navsys::entries::mag::MagSensor;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MagSps {
    entries: Vec<MagSensor>,
    raw_plots: Vec<RawPlot>,
}

impl MagSps {
    /// Read a file and attempt to deserialize a `MagSps` entry from it
    ///
    /// Return true if a valid header was deserialized
    pub fn file_is_valid(path: &Path) -> bool {
        let Ok(file) = fs::File::open(path) else {
            return false;
        };
        let mut reader = BufReader::new(file);
        // If 3 lines can be read successfully then it's valid
        MagSensor::from_reader(&mut reader).is_ok()
            && MagSensor::from_reader(&mut reader).is_ok()
            && MagSensor::from_reader(&mut reader).is_ok()
    }
}

impl fmt::Display for MagSps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.entries)
    }
}

impl LogEntry for MagSensor {
    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        // just a sanity check, it is definitely invalid if it is less than 10 characters
        if line.len() < 10 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Expected NavSysSps entry line but line is too short to be a NavSysSps entry. Line length={}, content={line}",
                    line.len()
                ),
            ));
        }
        let entry =
            Self::from_str(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok((entry, bytes_read))
    }

    fn timestamp_ns(&self) -> f64 {
        self.timestamp_ns()
    }
}

impl GitMetadata for MagSps {
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

impl Parseable for MagSps {
    const DESCRIPTIVE_NAME: &str = "MagSps";

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let (entries, bytes_read): (Vec<MagSensor>, usize) = parse_to_vec(reader);

        let mut raw_points_altitude: Vec<[f64; 2]> = Vec::new();

        for e in &entries {
            raw_points_altitude.push([e.timestamp_ns(), e.field_nanotesla()]);
        }

        let mut raw_plots = vec![RawPlot::new(
            "B-field [nT]".into(),
            raw_points_altitude,
            ExpectedPlotRange::Thousands,
        )];
        raw_plots.retain(|rp| {
            if rp.points().is_empty() {
                log::warn!("{} has no data", rp.name());
                false
            } else {
                true
            }
        });

        Ok((Self { entries, raw_plots }, bytes_read))
    }

    fn is_buf_valid(buf: &[u8]) -> bool {
        let mut reader = BufReader::new(buf);
        MagSensor::from_reader(&mut reader).is_ok()
    }
}

impl Plotable for MagSps {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.entries
            .first()
            .expect("No entries in MagSps, unable to get first timestamp")
            .timestamp()
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

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::{
        test_file_defs::frame_magnetometer::{
            FRAME_MAGNETOMETER_SPS_BYTES, frame_magnetometer, frame_magnetometer_sps,
        },
        *,
    };

    #[test]
    fn test_mag_sps_file_is_valid() {
        let is_valid = MagSps::file_is_valid(&frame_magnetometer_sps());
        assert!(is_valid);
    }

    #[test]
    fn test_wasp_sps_file_is_not_valid() {
        let is_valid = MagSps::file_is_valid(&wasp200_sps());
        assert!(!is_valid);
    }

    #[test]
    fn test_h5_file_is_not_valid() {
        let is_valid = MagSps::file_is_valid(&frame_magnetometer());
        assert!(!is_valid);
    }

    #[test]
    fn test_deserialize_mag_sps_file() {
        let mut frame_mag_sps = FRAME_MAGNETOMETER_SPS_BYTES;
        let (magsps, bytes_read) = MagSps::from_reader(&mut frame_mag_sps).unwrap();
        assert_eq!(bytes_read, 10599);
        assert_eq!(magsps.entries.len(), 273);
    }
}
