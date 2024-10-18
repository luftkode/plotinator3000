use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Display, PartialEq, Deserialize, Serialize)]
#[display("HE{id} {timestamp}: {altitude}")]
pub struct He {
    pub id: u8,
    timestamp: DateTime<Utc>,
    altitude: f64,
}

impl He {
    pub(crate) fn timestamp_ns(&self) -> f64 {
        self.timestamp
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range") as f64
    }
}

#[derive(Debug, Clone, Copy, Error)]
pub enum HeParseError {
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Invalid ID")]
    InvalidId,
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(#[from] chrono::ParseError),
    #[error("Invalid altitude")]
    InvalidAltitude,
}

impl FromStr for He {
    type Err = HeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 9 {
            return Err(HeParseError::InvalidFormat);
        }

        // Parse ID (format: "HE1" -> 1)
        let id = parts[0]
            .strip_prefix("HE")
            .and_then(|id| id.parse().ok())
            .ok_or(HeParseError::InvalidId)?;

        // Parse timestamp
        let timestamp_str = format!(
            "{} {} {} {} {} {} {}",
            parts[1], parts[2], parts[3], parts[4], parts[5], parts[6], parts[7]
        );
        let naive_dt = NaiveDateTime::parse_from_str(&timestamp_str, "%Y %m %d %H %M %S %3f")?;
        let timestamp = DateTime::from_naive_utc_and_offset(naive_dt, Utc);

        // Parse altitude
        let altitude = parts[8]
            .parse()
            .map_err(|_| HeParseError::InvalidAltitude)?;

        Ok(He {
            id,
            timestamp,
            altitude,
        })
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    const TEST_ENTRY_HE1: &str = "HE1 2024 10 03 12 52 42 448 99999.99";
    const TEST_ENTRY_HE2: &str = "HE2 2024 10 03 12 52 42 557 99999.99";
    const TEST_TWO_LINES_BOTH: &str = "HE1 2024 10 03 12 52 42 448 99999.99
HE2 2024 10 03 12 52 42 557 99999.99
";

    #[test]
    fn test_parse_navsys_he_entry() -> TestResult {
        // Test parsing HE1
        let he1 = He::from_str(TEST_ENTRY_HE1)?;
        assert_eq!(he1.id, 1);
        assert_eq!(he1.altitude, 99999.99);
        assert_eq!(
            he1.timestamp.naive_utc(),
            NaiveDateTime::parse_from_str("2024-10-03 12:52:42.448", "%Y-%m-%d %H:%M:%S.%3f")?
        );

        // Test parsing HE2
        let he2 = He::from_str(TEST_ENTRY_HE2)?;
        assert_eq!(he2.id, 2);
        assert_eq!(he2.altitude, 99999.99);
        assert_eq!(
            he2.timestamp.naive_utc(),
            NaiveDateTime::parse_from_str("2024-10-03 12:52:42.557", "%Y-%m-%d %H:%M:%S.%3f")?
        );

        Ok(())
    }

    #[test]
    fn test_misc() -> TestResult {
        // Test parsing multiple lines fails appropriately
        assert!(He::from_str(TEST_TWO_LINES_BOTH).is_err());

        // Test error cases
        assert!(matches!(
            He::from_str("invalid").unwrap_err(),
            HeParseError::InvalidFormat
        ));
        assert!(matches!(
            He::from_str("HEA 2024 10 03 12 52 42 448 99999.99").unwrap_err(),
            HeParseError::InvalidId
        ));
        assert!(matches!(
            He::from_str("HE1 2024 10 03 12 52 42 448 invalid").unwrap_err(),
            HeParseError::InvalidAltitude
        ));
        assert!(matches!(
            He::from_str("HE1 2024 13 03 12 52 42 448 99999.99").unwrap_err(),
            HeParseError::InvalidTimestamp(_)
        ));

        // Test Display formatting
        let he1 = He::from_str(TEST_ENTRY_HE1)?;
        assert_eq!(he1.to_string(), "HE1 2024-10-03 12:52:42.448 UTC: 99999.99");
        Ok(())
    }

    #[test]
    fn test_roundtrip_parsing() -> TestResult {
        let original = He {
            id: 1,
            timestamp: DateTime::from_naive_utc_and_offset(
                NaiveDateTime::parse_from_str("2024-10-03 12:52:42.448", "%Y-%m-%d %H:%M:%S.%3f")?,
                Utc,
            ),
            altitude: 99999.99,
        };

        let parsed = He::from_str(TEST_ENTRY_HE1)?;
        assert_eq!(original, parsed);

        Ok(())
    }
}
