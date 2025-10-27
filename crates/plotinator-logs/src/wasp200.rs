use std::{
    fs,
    io::{self, BufReader},
    path::Path,
    str::FromStr as _,
};

use anyhow::bail;
use chrono::{DateTime, Utc};
use plotinator_log_if::{parseable::Parseable, prelude::*};
use serde::{Deserialize, Serialize};

use crate::navsys::entries::he::AltimeterEntry;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Wasp200Sps {
    first_timestamp: DateTime<Utc>,
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

    fn timestamp_ns(&self) -> i64 {
        self.timestamp_ns() as i64
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

    fn from_reader(reader: &mut impl io::BufRead) -> anyhow::Result<(Self, usize)> {
        let (entries, bytes_read): (Vec<AltimeterEntry>, usize) = parse_to_vec(reader);

        let mut timestamps = Vec::with_capacity(entries.len());
        let mut altitudes = Vec::with_capacity(entries.len());
        let Some(first_timestamp) = entries.first().map(|e| e.timestamp()) else {
            bail!("Empty '{}' dataset", Self::DESCRIPTIVE_NAME);
        };

        for e in entries {
            if let Some(altitude) = e.altitude_m() {
                timestamps.push(e.timestamp_ns());
                altitudes.push(altitude);
            }
        }
        let Some(rawplot) = GeoSpatialDataBuilder::new("Wasp200")
            .timestamp(&timestamps)
            .altitude_from_laser(altitudes)
            .build_into_rawplot()?
        else {
            bail!(
                "Failed to build a rawplot from {} data",
                Self::DESCRIPTIVE_NAME
            )
        };

        Ok((
            Self {
                first_timestamp,
                raw_plots: vec![rawplot],
            },
            bytes_read,
        ))
    }

    fn is_buf_valid(buf: &[u8]) -> Result<(), String> {
        let mut reader = BufReader::new(buf);
        if let Err(e) = AltimeterEntry::from_reader(&mut reader) {
            Err(format!("Not a valid '{}': {e}", Self::DESCRIPTIVE_NAME))
        } else {
            Ok(())
        }
    }
}

impl Plotable for Wasp200Sps {
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
