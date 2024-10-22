use chrono::{DateTime, NaiveDateTime, Utc};
use derive_more::Display;
use getset::CopyGetters;
use serde::{Deserialize, Serialize};
use std::{
    num::{ParseFloatError, ParseIntError},
    str::FromStr,
};
use thiserror::Error;

#[derive(Debug, Clone, Display, PartialEq, Deserialize, Serialize, CopyGetters)]
#[display("GP{id} {timestamp}: {latitude:.5} {longitude:.5} {gp_time} {num_satellites} WGS84 {speed_kmh:.1} {hdop:.1} {vdop:.1} {pdop:.1} {altitude_above_mean_sea:.1}")]
pub struct Gps {
    pub id: u8,
    timestamp: DateTime<Utc>,
    #[getset(get_copy = "pub")]
    latitude: f64,
    #[getset(get_copy = "pub")]
    longitude: f64,
    // format: HH:MM:SS.<ms_fraction>
    #[getset(get = "pub")]
    gp_time: String,
    #[getset(get_copy = "pub")]
    num_satellites: u16,
    #[getset(get_copy = "pub")]
    speed_kmh: f32,
    #[getset(get_copy = "pub")]
    hdop: f32,
    #[getset(get_copy = "pub")]
    vdop: f32,
    #[getset(get_copy = "pub")]
    pdop: f32,
    #[getset(get_copy = "pub")]
    altitude_above_mean_sea: f32,
}

impl Gps {
    pub(crate) fn timestamp_ns(&self) -> f64 {
        self.timestamp
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range") as f64
    }

    /// Returns the difference between the entry timestamp (system time) and the timestamp received
    /// by the GPS in milliseconds
    pub(crate) fn gps_time_delta_ms(&self) -> f64 {
        // Parse the GPS time string (HH:MM:SS.000) into components
        let parts: Vec<&str> = self.gp_time.split(':').collect();
        let hours: i64 = parts[0].parse().unwrap_or(0);
        let minutes: i64 = parts[1].parse().unwrap_or(0);
        let seconds_parts: Vec<&str> = parts[2].split('.').collect();
        let seconds: i64 = seconds_parts[0].parse().unwrap_or(0);
        let millis: i64 = seconds_parts[1].parse().unwrap_or(0);

        // Create a NaiveDateTime for the same date but with GPS time
        let gps_timestamp = self
            .timestamp
            .date_naive()
            .and_hms_milli_opt(hours as u32, minutes as u32, seconds as u32, millis as u32)
            .expect("Invalid timestamp")
            .and_utc();

        // Calculate difference in milliseconds
        let system_ms = self.timestamp.timestamp_millis();
        let gps_ms = gps_timestamp.timestamp_millis();

        // Return the difference as a float
        (system_ms - gps_ms) as f64
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "It's a constructor with a lot of data that doesn't benefit from being grouped/wrapped into a struct"
    )]
    pub fn new(
        id: u8,
        timestamp: DateTime<Utc>,
        latitude: f64,
        longitude: f64,
        gp_time: String,
        num_satellites: u16,
        speed_kmh: f32,
        hdop: f32,
        vdop: f32,
        pdop: f32,
        altitude_above_mean_sea: f32,
    ) -> Self {
        Self {
            id,
            timestamp,
            latitude,
            longitude,
            gp_time,
            num_satellites,
            speed_kmh,
            hdop,
            vdop,
            pdop,
            altitude_above_mean_sea,
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum GpsError {
    #[error("Invalid format")]
    Format,
    #[error("Invalid ID")]
    Id,
    #[error("Invalid timestamp: {0}")]
    Timestamp(#[from] chrono::ParseError),
    #[error("Invalid latitude: {0}")]
    Latitude(String),
    #[error("Invalid longitude: {0}")]
    Longitude(String),
    #[error("Invalid GPS time format: {0}")]
    GpsTime(String),
    #[error("Invalid number of satellites: {0}")]
    Satellites(String),
    #[error("Invalid coordinate system (expected WGS84)")]
    CoordinateSystem,
    #[error("Invalid Speed: {0}")]
    Speed(String),
    #[error("Invalid HDOP: {0}")]
    Hdop(String),
    #[error("Invalid VDOP: {0}")]
    Vdop(String),
    #[error("Invalid PDOP: {0}")]
    Pdop(String),
    #[error("Invalid Altitude: {0}")]
    Altitude(String),
}

impl FromStr for Gps {
    type Err = GpsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 18 {
            return Err(GpsError::Format);
        }

        // Parse ID (format: "GP1" -> 1)
        let id = parts[0]
            .strip_prefix("GP")
            .and_then(|id| id.parse().ok())
            .ok_or(GpsError::Id)?;

        // Parse timestamp
        let timestamp_str = format!(
            "{} {} {} {} {} {} {}",
            parts[1], parts[2], parts[3], parts[4], parts[5], parts[6], parts[7]
        );
        let timestamp =
            NaiveDateTime::parse_from_str(&timestamp_str, "%Y %m %d %H %M %S %3f")?.and_utc();

        // Parse latitude and longitude
        let latitude = parts[8]
            .parse()
            .map_err(|e: ParseFloatError| GpsError::Latitude(e.to_string()))?;
        let longitude = parts[9]
            .parse()
            .map_err(|e: ParseFloatError| GpsError::Longitude(e.to_string()))?;

        // Parse GPS time
        let gp_time = parts[10].to_owned();
        if !gp_time.matches(':').count() == 2 {
            return Err(GpsError::GpsTime(format!("Invalid format: {gp_time}")));
        }

        // Parse number of satellites
        let num_satellites = parts[11]
            .parse()
            .map_err(|e: ParseIntError| GpsError::Satellites(e.to_string()))?;

        // Verify coordinate system
        if parts[12] != "WGS84" {
            return Err(GpsError::CoordinateSystem);
        }

        // Parse DOP values and other metrics
        let speed = parts[13]
            .parse()
            .map_err(|e: ParseFloatError| GpsError::Speed(e.to_string()))?;
        let hdop = parts[14]
            .parse()
            .map_err(|e: ParseFloatError| GpsError::Hdop(e.to_string()))?;
        let vdop = parts[15]
            .parse()
            .map_err(|e: ParseFloatError| GpsError::Vdop(e.to_string()))?;
        let pdop = parts[16]
            .parse()
            .map_err(|e: ParseFloatError| GpsError::Pdop(e.to_string()))?;
        let altitude = parts[17]
            .parse()
            .map_err(|e: ParseFloatError| GpsError::Altitude(e.to_string()))?;

        Ok(Self::new(
            id,
            timestamp,
            latitude,
            longitude,
            gp_time,
            num_satellites,
            speed,
            hdop,
            vdop,
            pdop,
            altitude,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    const TEST_ENTRY_GP1: &str = "GP1 2024 10 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2";
    const TEST_ENTRY_GP2: &str = "GP2 2024 10 03 12 52 43 025 5347.57764 933.01312 12:52:43.000 17 WGS84 0.0 0.9 1.2 1.5 -0.1";
    const TEST_TWO_LINES_BOTH: &str =
        "GP1 2024 10 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2
GP2 2024 10 03 12 52 43 025 5347.57764 933.01312 12:52:43.000 17 WGS84 0.0 0.9 1.2 1.5 -0.1
";

    #[test]
    fn test_parse_gps_entry() -> TestResult {
        let gp1 = Gps::from_str(TEST_ENTRY_GP1)?;
        assert_eq!(gp1.id, 1);
        assert_eq!(gp1.latitude, 5347.57959);
        assert_eq!(gp1.longitude, 933.01392);
        assert_eq!(gp1.gp_time, "12:52:43.000");
        assert_eq!(gp1.num_satellites, 16);
        assert_eq!(gp1.speed_kmh, 0.0);
        assert_eq!(gp1.hdop, 0.8);
        assert_eq!(gp1.vdop, 1.3);
        assert_eq!(gp1.pdop, 1.5);
        assert_eq!(gp1.altitude_above_mean_sea, 0.2);
        assert_eq!(
            gp1.timestamp.naive_utc(),
            NaiveDateTime::parse_from_str("2024-10-03 12:52:42.994", "%Y-%m-%d %H:%M:%S.%3f")?
        );

        let gp2 = Gps::from_str(TEST_ENTRY_GP2)?;
        assert_eq!(gp2.id, 2);
        assert_eq!(gp2.latitude, 5347.57764);
        assert_eq!(gp2.longitude, 933.01312);
        assert_eq!(gp2.gp_time, "12:52:43.000");
        assert_eq!(gp2.num_satellites, 17);
        assert_eq!(gp2.speed_kmh, 0.0);
        assert_eq!(gp2.hdop, 0.9);
        assert_eq!(gp2.vdop, 1.2);
        assert_eq!(gp2.pdop, 1.5);
        assert_eq!(gp2.altitude_above_mean_sea, -0.1);

        Ok(())
    }

    #[test]
    fn test_error_cases() {
        // Test invalid format
        assert!(matches!(
            Gps::from_str("invalid").unwrap_err(),
            GpsError::Format
        ));

        // Test invalid ID
        assert!(matches!(
            Gps::from_str("GPA 2024 10 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2").unwrap_err(),
            GpsError::Id
        ));

        // Test invalid coordinate system
        assert!(matches!(
            Gps::from_str("GP1 2024 10 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 NAD83 0.0 0.8 1.3 1.5 0.2").unwrap_err(),
            GpsError::CoordinateSystem
        ));

        // Test invalid timestamp
        assert!(matches!(
            Gps::from_str("GP1 2024 13 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2").unwrap_err(),
            GpsError::Timestamp(_)
        ));

        // Test multi-line parsing
        assert!(Gps::from_str(TEST_TWO_LINES_BOTH).is_err());
    }

    #[test]
    fn test_display_formatting() -> TestResult {
        let gp1 = Gps::from_str(TEST_ENTRY_GP1)?;
        assert_eq!(
            gp1.to_string(),
            "GP1 2024-10-03 12:52:42.994 UTC: 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2"
        );
        Ok(())
    }

    #[test]
    fn test_timestamp_ns() -> TestResult {
        let gp1 = Gps::from_str(TEST_ENTRY_GP1)?;
        let expected_ns = (gp1.timestamp.timestamp() as f64) * 1e9
            + (gp1.timestamp.timestamp_subsec_nanos() as f64);
        assert_eq!(gp1.timestamp_ns(), expected_ns);
        Ok(())
    }

    #[test]
    fn test_gps_time_delta() -> TestResult {
        let gps = Gps::from_str("GP1 2024 10 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2")?;
        // System time is 42.994, GPS time is 43.000
        // Expected delta: 42.994 - 43.000 = -0.006 seconds = -6 milliseconds
        let gps_time_delta_ms = gps.gps_time_delta_ms();
        assert_eq!(gps_time_delta_ms, -6.);
        Ok(())
    }
}
