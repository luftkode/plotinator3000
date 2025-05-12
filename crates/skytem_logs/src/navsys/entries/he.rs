use std::{num::ParseFloatError, str::FromStr};

use chrono::{DateTime, NaiveDateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Display, PartialEq, Deserialize, Serialize)]
#[display("HE{id} {timestamp}: {altitude_m:?}")]
pub struct AltimeterEntry {
    pub id: u8,
    timestamp: DateTime<Utc>,
    altitude_m: Option<f64>,
}

impl AltimeterEntry {
    /// This value is placeholder for an invalid reading
    pub(crate) const INVALID_VALUE: &str = "99999.99";

    pub(crate) fn timestamp_ns(&self) -> f64 {
        self.timestamp
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range") as f64
    }

    pub(crate) fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Altitude in meters above mean sea level, return `None` if the altitude was the invalid value
    pub(crate) fn altitude_m(&self) -> Option<f64> {
        self.altitude_m
    }
}

#[derive(Debug, Clone, Error)]
pub enum AltimeterParseError {
    #[error("Invalid format: {0}")]
    Format(String),
    #[error("Invalid ID: {0}")]
    Id(String),
    #[error("Invalid timestamp: {0}")]
    Timestamp(#[from] chrono::ParseError),
    #[error("Invalid altitude: {0}")]
    Altitude(String),
}

impl FromStr for AltimeterEntry {
    type Err = AltimeterParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 9 {
            return Err(AltimeterParseError::Format(
                "Separating line by whitespace did not return 9 parts".to_owned(),
            ));
        }

        // Parse ID (format: "HE1" -> 1)
        let id = parts[0]
            .strip_prefix("HE")
            .and_then(|id| id.parse().ok())
            .ok_or(AltimeterParseError::Id(format!(
                "Expected prefix 'HE', got '{}'",
                parts[0]
            )))?;

        // Parse timestamp
        let timestamp_str = format!(
            "{} {} {} {} {} {} {}",
            parts[1], parts[2], parts[3], parts[4], parts[5], parts[6], parts[7]
        );
        let naive_dt = NaiveDateTime::parse_from_str(&timestamp_str, "%Y %m %d %H %M %S %3f")?;
        let timestamp = DateTime::from_naive_utc_and_offset(naive_dt, Utc);

        // Parse altitude
        let altitude = if parts[8] == Self::INVALID_VALUE {
            None
        } else {
            Some(
                parts[8]
                    .parse()
                    .map_err(|e: ParseFloatError| AltimeterParseError::Altitude(e.to_string()))?,
            )
        };
        Ok(Self {
            id,
            timestamp,
            altitude_m: altitude,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_util::*;

    const TEST_ENTRY_HE1: &str = "HE1 2024 10 03 12 52 42 448 99999.99";
    const TEST_ENTRY_HE2: &str = "HE2 2024 10 03 12 52 42 557 123.99";
    const TEST_TWO_LINES_BOTH: &str = "HE1 2024 10 03 12 52 42 448 99999.99
HE2 2024 10 03 12 52 42 557 99999.99
";

    #[test]
    fn test_parse_navsys_he_entry() -> TestResult {
        // Test parsing HE1
        let he1 = AltimeterEntry::from_str(TEST_ENTRY_HE1)?;
        assert_eq!(he1.id, 1);
        assert_eq!(he1.altitude_m, None);
        assert_eq!(
            he1.timestamp.naive_utc(),
            NaiveDateTime::parse_from_str("2024-10-03 12:52:42.448", "%Y-%m-%d %H:%M:%S.%3f")?
        );

        // Test parsing HE2
        let he2 = AltimeterEntry::from_str(TEST_ENTRY_HE2)?;
        assert_eq!(he2.id, 2);
        assert_eq!(he2.altitude_m, Some(123.99));
        assert_eq!(
            he2.timestamp.naive_utc(),
            NaiveDateTime::parse_from_str("2024-10-03 12:52:42.557", "%Y-%m-%d %H:%M:%S.%3f")?
        );

        Ok(())
    }

    #[test]
    fn test_misc() -> TestResult {
        // Test parsing multiple lines fails appropriately
        assert!(AltimeterEntry::from_str(TEST_TWO_LINES_BOTH).is_err());

        // Test error cases
        assert!(matches!(
            AltimeterEntry::from_str("invalid").unwrap_err(),
            AltimeterParseError::Format(_)
        ));
        assert!(matches!(
            AltimeterEntry::from_str("HEA 2024 10 03 12 52 42 448 99999.99").unwrap_err(),
            AltimeterParseError::Id(_)
        ));
        assert!(matches!(
            AltimeterEntry::from_str("HE1 2024 10 03 12 52 42 448 invalid").unwrap_err(),
            AltimeterParseError::Altitude(_)
        ));
        assert!(matches!(
            AltimeterEntry::from_str("HE1 2024 13 03 12 52 42 448 99999.99").unwrap_err(),
            AltimeterParseError::Timestamp(_)
        ));

        // Test Display formatting
        let he1 = AltimeterEntry::from_str(TEST_ENTRY_HE1)?;
        assert_eq!(he1.to_string(), "HE1 2024-10-03 12:52:42.448 UTC: None");
        Ok(())
    }

    #[test]
    fn test_roundtrip_parsing() -> TestResult {
        let original = AltimeterEntry {
            id: 1,
            timestamp: DateTime::from_naive_utc_and_offset(
                NaiveDateTime::parse_from_str("2024-10-03 12:52:42.448", "%Y-%m-%d %H:%M:%S.%3f")?,
                Utc,
            ),
            altitude_m: None,
        };

        let parsed = AltimeterEntry::from_str(TEST_ENTRY_HE1)?;
        assert_eq!(original, parsed);

        Ok(())
    }
}
