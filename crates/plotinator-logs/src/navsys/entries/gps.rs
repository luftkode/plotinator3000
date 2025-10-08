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
#[display(
    "GP{id} {timestamp}: {latitude:.5} {longitude:.5} {gp_time} {num_satellites} WGS84 {speed_kmh:.1} {hdop:.1} {vdop:.1} {pdop:.1} {altitude_above_mean_sea:.1}"
)]
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

/// Convert NMEA-style coordinate (ddmm.mmmm or dddmm.mmmm) into decimal degrees.
fn nmea_to_decimal(coord: f64) -> f64 {
    if coord.is_nan() {
        return f64::NAN;
    }

    let sign = if coord.is_sign_negative() { -1.0 } else { 1.0 };
    let abs = coord.abs();

    let int_part = abs.trunc();
    let frac_part = abs.fract();

    // minutes are last two digits of int_part + fractional minutes
    let minutes = (int_part % 100.0) + frac_part;

    // degrees are everything before the last two digits
    let degrees = (int_part / 100.0).trunc(); // safe because abs >= 0

    sign * (degrees + (minutes / 60.0))
}

impl Gps {
    pub(crate) fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

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

    /// Returns latitude in decimal degrees
    pub(crate) fn latitude_deg(&self) -> f64 {
        nmea_to_decimal(self.latitude)
    }

    /// Returns longitude in decimal degrees
    pub(crate) fn longitude_deg(&self) -> f64 {
        nmea_to_decimal(self.longitude)
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

    #[allow(clippy::too_many_lines, reason = "Long but simple")]
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
        let lat_str = parts[8];
        let latitude = if lat_str == "NaN" {
            f64::NAN
        } else {
            lat_str
                .parse()
                .map_err(|e: ParseFloatError| GpsError::Latitude(e.to_string()))?
        };
        let lon_str = parts[9];
        let longitude = if lon_str == "NaN" {
            f64::NAN
        } else {
            lon_str
                .parse()
                .map_err(|e: ParseFloatError| GpsError::Longitude(e.to_string()))?
        };

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
        let speed_str = parts[13];
        let speed = if speed_str == "NaN" {
            f32::NAN
        } else {
            speed_str
                .parse()
                .map_err(|e: ParseFloatError| GpsError::Speed(e.to_string()))?
        };
        let hdop_str = parts[14];
        let hdop = if hdop_str == "NaN" {
            f32::NAN
        } else {
            hdop_str
                .parse()
                .map_err(|e: ParseFloatError| GpsError::Hdop(e.to_string()))?
        };
        let vdop_str = parts[15];
        let vdop = if vdop_str == "NaN" {
            f32::NAN
        } else {
            vdop_str
                .parse()
                .map_err(|e: ParseFloatError| GpsError::Vdop(e.to_string()))?
        };
        let pdop_str = parts[16];
        let pdop = if pdop_str == "NaN" {
            f32::NAN
        } else {
            pdop_str
                .parse()
                .map_err(|e: ParseFloatError| GpsError::Pdop(e.to_string()))?
        };
        let altitude_str = parts[17];
        let altitude = if altitude_str == "NaN" {
            f32::NAN
        } else {
            altitude_str
                .parse()
                .map_err(|e: ParseFloatError| GpsError::Altitude(e.to_string()))?
        };

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
    use plotinator_test_util::*;

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
        let gps = Gps::from_str(
            "GP1 2024 10 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2",
        )?;
        // System time is 42.994, GPS time is 43.000
        // Expected delta: 42.994 - 43.000 = -0.006 seconds = -6 milliseconds
        let gps_time_delta_ms = gps.gps_time_delta_ms();
        assert_eq!(gps_time_delta_ms, -6.);
        Ok(())
    }

    #[test]
    fn test_gps_from_str_certus() {
        let certus_sps = "GP1 2025 06 30 11 38 11 400 56.21767 10.14784 11:38:11.400 0 WGS84 0.0 0.0 0.0 0.0 59.6";
        let navsys_sps = "GP1 2024 10 03 12 52 42 994 5347.57959 933.01392 12:52:43.000 16 WGS84 0.0 0.8 1.3 1.5 0.2";
        assert!(Gps::from_str(navsys_sps).is_ok());
        assert!(Gps::from_str(certus_sps).is_ok());
    }

    #[test]
    fn test_gps_from_str_data_binder_conv() {
        let data_binder_conv_sps = "GP2 2025 09 01 08 23 09 137 5613.06620 1008.86800 08:23:09.000 10 WGS84 0.0 0.9 1.4 1.7 66.6";
        assert!(Gps::from_str(data_binder_conv_sps).is_ok());
    }

    #[test]
    fn test_nmea_to_decimal_conversion() {
        // 49°16.45′ = 49 + 16.45 / 60 = 49.274166...
        let nmea_val = 4916.45;
        let expected = 49.274166;
        let result = nmea_to_decimal(nmea_val);
        let deviation_from_expected = (result - expected).abs();
        assert!(deviation_from_expected < 1e-6, "latitude conversion failed");

        // 123°19.943′ = 123 + 19.943 / 60 = 123.332383...
        let nmea_val = 12319.943;
        let expected = 123.332383;
        let deviation_from_expected = (nmea_to_decimal(nmea_val) - expected).abs();
        assert!(
            deviation_from_expected < 1e-6,
            "longitude conversion failed"
        );

        // 12°34.5678′ = 12 + 34.5678 / 60 = 12.57613...
        let nmea_val = 1234.5678;
        let expected = 12.57613;
        let deviation_from_expected = (nmea_to_decimal(nmea_val) - expected).abs();
        assert!(deviation_from_expected < 1e-10, "fractional minutes failed");

        assert_eq!(nmea_to_decimal(0.0), 0.0);

        // Negative coordinates (e.g. western or southern hemisphere)
        // -49°16.45′ = -(49 + 16.45 / 60) = -49.274166...
        let nmea_val = -4916.45;
        let expected = -49.274166;
        let deviation_from_expected = (nmea_to_decimal(nmea_val) - expected).abs();
        assert!(
            deviation_from_expected < 1e-6,
            "negative latitude conversion failed"
        );

        // -123°19.943′ = -(123 + 19.943 / 60) = -123.332383...
        let nmea_val = -12319.943;
        let expected = -123.332383;
        let deviation_from_expected = (nmea_to_decimal(nmea_val) - expected).abs();
        assert!(
            deviation_from_expected < 1e-6,
            "negative longitude conversion failed"
        );

        // High-precision edge case
        let nmea_val = 4559.9999;
        let expected = 45.9999983;
        let deviation_from_expected = (nmea_to_decimal(nmea_val) - expected).abs();
        assert!(deviation_from_expected < 1e-6, "high-precision case failed");

        // NaN should return NaN
        let result = nmea_to_decimal(f64::NAN);
        assert!(result.is_nan(), "NaN handling failed");
    }
}
