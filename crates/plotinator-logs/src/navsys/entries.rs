use chrono::{DateTime, Utc};
use gps::Gps;
use he::AltimeterEntry;
use mag::MagSensor;
use plotinator_log_if::log::LogEntry;
use serde::{Deserialize, Serialize};
use std::{fmt, io, str::FromStr as _};
use tl::InclinometerEntry;

pub mod gps;
pub mod he;
pub mod mag;
pub mod tl;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum NavSysSpsEntry {
    HE1(AltimeterEntry),
    HE2(AltimeterEntry),
    HE3(AltimeterEntry), // wasp200
    HEx(AltimeterEntry), // fallback for none of the other HEs
    TL1(InclinometerEntry),
    TL2(InclinometerEntry),
    TL3(InclinometerEntry), // Njord INS
    TLx(InclinometerEntry), // Fallback
    GP1(Gps),
    GP2(Gps),
    GP3(Gps), // Njord INS
    GPx(Gps), // Fallback
    MA1(MagSensor),
    MA2(MagSensor),
    MAx(MagSensor), // Fallback
}

impl fmt::Display for NavSysSpsEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HE1(he) | Self::HE2(he) | Self::HE3(he) | Self::HEx(he) => write!(f, "{he}"),
            Self::TL1(tl) | Self::TL2(tl) | Self::TL3(tl) | Self::TLx(tl) => write!(f, "{tl}"),
            Self::GP1(gps) | Self::GP2(gps) | Self::GP3(gps) | Self::GPx(gps) => write!(f, "{gps}"),
            Self::MA1(ma) | Self::MA2(ma) | Self::MAx(ma) => write!(f, "{ma}"),
        }
    }
}

impl NavSysSpsEntry {
    pub(crate) fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::HE1(he) | Self::HE2(he) | Self::HE3(he) | Self::HEx(he) => he.timestamp(),
            Self::TL1(tl) | Self::TL2(tl) | Self::TL3(tl) | Self::TLx(tl) => tl.timestamp(),
            Self::GP1(gps) | Self::GP2(gps) | Self::GP3(gps) | Self::GPx(gps) => gps.timestamp(),
            Self::MA1(ma) | Self::MA2(ma) | Self::MAx(ma) => ma.timestamp(),
        }
    }
}

impl LogEntry for NavSysSpsEntry {
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
        let first_three_chars = &line[..3];
        let entry: Self = match first_three_chars {
            h if h.starts_with("HE") => {
                let he = AltimeterEntry::from_str(&line)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                match he.id {
                    1 => Self::HE1(he),
                    2 => Self::HE2(he),
                    3 => Self::HE3(he),
                    _ => Self::HEx(he),
                }
            }
            tl if tl.starts_with("TL") => {
                let tl = InclinometerEntry::from_str(&line)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                match tl.id {
                    1 => Self::TL1(tl),
                    2 => Self::TL2(tl),
                    3 => Self::TL3(tl),
                    _ => Self::TLx(tl),
                }
            }
            gp if gp.starts_with("GP") => {
                let gps = Gps::from_str(&line)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                match gps.id {
                    1 => Self::GP1(gps),
                    2 => Self::GP2(gps),
                    3 => Self::GP3(gps),
                    _ => Self::GPx(gps),
                }
            }
            m if m.starts_with("MA") => {
                let mag = MagSensor::from_str(&line)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                match mag.id {
                    1 => Self::MA1(mag),
                    2 => Self::MA2(mag),
                    _ => Self::MAx(mag),
                }
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Expected a NavSysSps entry ID, got: {first_three_chars}"),
                ));
            }
        };
        Ok((entry, bytes_read))
    }

    fn timestamp_ns(&self) -> f64 {
        match self {
            Self::HE1(he) | Self::HE2(he) | Self::HE3(he) | Self::HEx(he) => he.timestamp_ns(),
            Self::TL1(tl) | Self::TL2(tl) | Self::TL3(tl) | Self::TLx(tl) => tl.timestamp_ns(),
            Self::GP1(gps) | Self::GP2(gps) | Self::GP3(gps) | Self::GPx(gps) => gps.timestamp_ns(),
            Self::MA1(ma) | Self::MA2(ma) | Self::MAx(ma) => ma.timestamp_ns(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_log_if::util::parse_to_vec;

    const TEST_ENTRIES: &str = "HE1 2024 10 03 12 52 42 448 99999.99
TL2 2024 10 03 12 52 42 542 2.34 0.58
HE2 2024 10 03 12 52 42 557 99999.99
TL1 2024 10 03 12 52 42 838 2.15 0.24
GP1 2024 10 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2
GP2 2024 10 03 12 52 43 025 5347.57764 933.01312 12:52:43.000 17 WGS84 0.0 0.9 1.2 1.5 -0.1
MA1 2024 10 03 12 52 55 747 49894.8659
";

    #[test]
    fn test_parse_navsys_entries() {
        let (entries, bytes_read): (Vec<NavSysSpsEntry>, usize) =
            parse_to_vec(&mut TEST_ENTRIES.as_bytes());

        assert_eq!(entries.len(), 7);
        assert_eq!(bytes_read, 372);
    }
}
