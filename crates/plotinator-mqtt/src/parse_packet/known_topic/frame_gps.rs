use chrono::{DateTime, Utc};
use plotinator_log_if::prelude::{GeoAltitude, GeoPoint};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct FrameGpsPacket {
    pub(crate) gps_nr: u8,
    pub(crate) timestamp: u64,
    /// Fix type. 0 is no fix, 3 is 3D fix
    pub(crate) mode: u8,
    pub(crate) gps_status: FrameGpsStatus,
    pub(crate) position: FrameGpsPosition,
    pub(crate) speed: Option<f32>,
    pub(crate) gps_time: DateTime<Utc>,
}

impl FrameGpsPacket {
    /// Try to extract a [`GeoPoint`] from the [`FrameGpsPacket`] if it has at least longitude and latitude
    #[inline]
    pub(crate) fn maybe_get_geopoint(&self) -> Option<GeoPoint> {
        let ts = self.gps_time.timestamp_nanos_opt()? as f64;
        let lat = self.position.lat?;
        let lon = self.position.lon?;
        let mut geo = GeoPoint::new(ts, (lat.into(), lon.into()));
        if let Some(alt) = self.position.alt {
            let geo_alt = GeoAltitude::Gnss(alt.into());
            geo = geo.with_altitude(geo_alt);
        }
        if let Some(spd) = self.speed {
            geo = geo.with_speed(spd.into());
        }

        Some(geo)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct FrameGpsPosition {
    pub(crate) lat: Option<f32>,
    pub(crate) lon: Option<f32>,
    pub(crate) alt: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FrameGpsStatus {
    pub(crate) hdop: Option<f32>,
    pub(crate) pdop: Option<f32>,
    pub(crate) vdop: Option<f32>,
    pub(crate) satellites: u8,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use crate::parse_packet::known_topic::KnownTopic;

    use super::*;
    use plotinator_test_util::*;

    #[test]
    fn test_parse_packet() -> TestResult {
        let payload = r#"{"gps_nr":2,"timestamp":1757321363140914802,"mode":3,"gps_status":{"hdop":0.8,"vdop":1.5,"pdop":1.7,"satellites":10},"gps_time":"2025-09-08T08:49:23.000Z","position":{"lat":56.217778333,"lon":10.147778333,"alt":66.62},"speed":0.052}"#;
        let decoded_json: FrameGpsPacket = serde_json::from_str(payload)?;
        let known_topic = KnownTopic::from_str(&KnownTopic::TcFrameGps.to_string())?;
        let mqtt_data = known_topic.parse_packet(payload)?.unwrap();

        insta::assert_debug_snapshot!(decoded_json);
        insta::assert_debug_snapshot!(mqtt_data);
        Ok(())
    }
}
