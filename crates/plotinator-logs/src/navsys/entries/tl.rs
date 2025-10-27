use chrono::{DateTime, NaiveDateTime, Utc};
use derive_more::Display;
use getset::CopyGetters;
use serde::{Deserialize, Serialize};
use std::{num::ParseFloatError, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Display, CopyGetters)]
#[display("TL{id} {timestamp}: {pitch_angle_degrees} {roll_angle_degrees}")]
pub struct InclinometerEntry {
    pub id: u8,
    timestamp: DateTime<Utc>,
    #[getset(get_copy = "pub")]
    pitch_angle_degrees: f64,
    #[getset(get_copy = "pub")]
    roll_angle_degrees: f64,
}

impl InclinometerEntry {
    pub(crate) fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    pub(crate) fn timestamp_ns(&self) -> i64 {
        self.timestamp
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range")
    }
}

#[derive(Debug, Clone, Error)]
pub enum InclinometerEntryError {
    #[error("Invalid format")]
    Format(String),
    #[error("Invalid ID")]
    Id(String),
    #[error("Invalid timestamp: {0}")]
    Timestamp(#[from] chrono::ParseError),
    #[error("Invalid pitch angle")]
    PitchAngle(String),
    #[error("Invalid roll angle")]
    RollAngle(String),
}

impl FromStr for InclinometerEntry {
    type Err = InclinometerEntryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 10 {
            return Err(InclinometerEntryError::Format(
                "Separating line by whitespace did not return 10 parts".to_owned(),
            ));
        }

        // Parse ID (format: "TL1" -> 1)
        let id = parts[0]
            .strip_prefix("TL")
            .and_then(|id| id.parse().ok())
            .ok_or(InclinometerEntryError::Id(format!(
                "Expected prefix 'TL', got '{}'",
                parts[0]
            )))?;

        // Parse timestamp
        let timestamp_str = format!(
            "{} {} {} {} {} {} {}",
            parts[1], parts[2], parts[3], parts[4], parts[5], parts[6], parts[7]
        );
        let naive_dt = NaiveDateTime::parse_from_str(&timestamp_str, "%Y %m %d %H %M %S %3f")?;
        let timestamp = DateTime::from_naive_utc_and_offset(naive_dt, Utc);

        let pitch_angle_degrees = parts[8]
            .parse()
            .map_err(|e: ParseFloatError| InclinometerEntryError::PitchAngle(e.to_string()))?;

        let roll_angle_degrees = parts[9]
            .parse()
            .map_err(|e: ParseFloatError| InclinometerEntryError::RollAngle(e.to_string()))?;

        Ok(Self {
            id,
            timestamp,
            pitch_angle_degrees,
            roll_angle_degrees,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_test_util::*;

    const TEST_ENTRY_TL1: &str = "TL1 2024 10 03 12 52 42 838 2.15 0.24";
    const TEST_ENTRY_TL2: &str = "TL2 2024 10 03 12 52 42 542 2.34 0.58";
    const TEST_TWO_LINES_BOTH: &str = "TL1 2024 10 03 12 52 42 838 2.15 0.24
TL2 2024 10 03 12 52 42 542 2.34 0.58
";

    #[test]
    fn test_parse_inclinometer_entry() -> TestResult {
        // Test parsing TL1
        let tl1 = InclinometerEntry::from_str(TEST_ENTRY_TL1)?;
        assert_eq!(tl1.id, 1);
        assert_eq!(tl1.pitch_angle_degrees, 2.15);
        assert_eq!(tl1.roll_angle_degrees, 0.24);
        assert_eq!(
            tl1.timestamp.naive_utc(),
            NaiveDateTime::parse_from_str("2024-10-03 12:52:42.838", "%Y-%m-%d %H:%M:%S.%3f")?
        );

        // Test parsing TL2
        let tl2 = InclinometerEntry::from_str(TEST_ENTRY_TL2)?;
        assert_eq!(tl2.id, 2);
        assert_eq!(tl2.pitch_angle_degrees, 2.34);
        assert_eq!(tl2.roll_angle_degrees, 0.58);
        assert_eq!(
            tl2.timestamp.naive_utc(),
            NaiveDateTime::parse_from_str("2024-10-03 12:52:42.542", "%Y-%m-%d %H:%M:%S.%3f")?
        );

        // Test parsing multiple lines fails appropriately
        assert!(InclinometerEntry::from_str(TEST_TWO_LINES_BOTH).is_err());

        Ok(())
    }

    #[test]
    fn test_error_cases() {
        // Test invalid format
        assert!(matches!(
            InclinometerEntry::from_str("invalid").unwrap_err(),
            InclinometerEntryError::Format(_)
        ));

        // Test invalid ID
        assert!(matches!(
            InclinometerEntry::from_str("TLA 2024 10 03 12 52 42 838 2.15 0.24").unwrap_err(),
            InclinometerEntryError::Id(_)
        ));

        // Test invalid angle
        assert!(matches!(
            InclinometerEntry::from_str("TL1 2024 10 03 12 52 42 838 invalid 0.24").unwrap_err(),
            InclinometerEntryError::PitchAngle(_)
        ));

        // Test invalid uncertainty
        assert!(matches!(
            InclinometerEntry::from_str("TL1 2024 10 03 12 52 42 838 2.15 invalid").unwrap_err(),
            InclinometerEntryError::RollAngle(_)
        ));

        // Test invalid timestamp
        assert!(matches!(
            InclinometerEntry::from_str("TL1 2024 13 03 12 52 42 838 2.15 0.24").unwrap_err(),
            InclinometerEntryError::Timestamp(_)
        ));
    }

    #[test]
    fn test_display_formatting() -> TestResult {
        let tl1 = InclinometerEntry::from_str(TEST_ENTRY_TL1)?;
        assert_eq!(
            tl1.to_string(),
            "TL1 2024-10-03 12:52:42.838 UTC: 2.15 0.24"
        );
        Ok(())
    }

    #[test]
    fn test_timestamp_ns() -> TestResult {
        let tl1 = InclinometerEntry::from_str(TEST_ENTRY_TL1)?;
        let expected_ns = (tl1.timestamp.timestamp()) * 1_000_000_000
            + (tl1.timestamp.timestamp_subsec_nanos() as i64);
        assert_eq!(tl1.timestamp_ns(), expected_ns);
        Ok(())
    }
}
