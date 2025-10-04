use std::{
    fmt, fs,
    io::{self, BufReader},
    path::Path,
    str::FromStr as _,
};

use chrono::{DateTime, Utc};
use plotinator_log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};

use crate::navsys::entries::he::AltimeterEntry;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Wasp200Sps {
    entries: Vec<AltimeterEntry>,
    raw_plots: Vec<RawPlot>,
}

impl Wasp200Sps {
    /// Read a file and attempt to deserialize a `Wasp200Sps` entry from it
    ///
    /// Return true if a valid header was deserialized
    pub fn file_is_valid(path: &Path) -> bool {
        let Ok(file) = fs::File::open(path) else {
            return false;
        };
        let mut reader = BufReader::new(file);
        if let Err(e) = AltimeterEntry::from_reader(&mut reader) {
            log::debug!("Not a valid NavSys HE line: {e}");
            false
        } else {
            true
        }
    }
}

impl fmt::Display for Wasp200Sps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.entries)
    }
}

impl LogEntry for AltimeterEntry {
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

    fn timestamp_ns(&self) -> f64 {
        self.timestamp_ns()
    }
}

impl GitMetadata for Wasp200Sps {
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

impl Parseable for Wasp200Sps {
    const DESCRIPTIVE_NAME: &str = "Wasp200Sps";

    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        let (entries, bytes_read): (Vec<AltimeterEntry>, usize) = parse_to_vec(reader);

        let mut raw_points_altitude: Vec<[f64; 2]> = Vec::new();

        for e in &entries {
            if let Some(altitude) = e.altitude_m() {
                raw_points_altitude.push([e.timestamp_ns(), altitude]);
            }
        }

        let mut raw_plots = vec![RawPlotCommon::new(
            "Wasp200 Altitude [M]".into(),
            raw_points_altitude,
            ExpectedPlotRange::OneToOneHundred,
        )];
        raw_plots.retain(|rp| {
            if rp.points().is_empty() {
                log::warn!("{} has no data", rp.name());
                false
            } else {
                true
            }
        });
        let raw_plots = raw_plots.into_iter().map(Into::into).collect();

        Ok((Self { entries, raw_plots }, bytes_read))
    }

    fn is_buf_valid(buf: &[u8]) -> bool {
        let mut reader = BufReader::new(buf);
        AltimeterEntry::from_reader(&mut reader).is_ok()
    }
}

impl Plotable for Wasp200Sps {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.raw_plots
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.entries
            .first()
            .expect("No entries in Wasp200Sps, unable to get first timestamp")
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
