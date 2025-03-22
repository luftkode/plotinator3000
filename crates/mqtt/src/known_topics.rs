use std::str::FromStr;

use egui_plot::PlotPoint;
use serde::Deserialize;
use strum_macros::{Display, EnumString};

use crate::data::MqttPoint;

pub(crate) fn now_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as f64
}

/// Known topics can have custom payloads that have an associated known packet structure
/// which allows recognizing and parsing them appropriately
#[derive(EnumString, Display)]
pub enum KnownTopic {
    #[strum(serialize = "debug/sensors/temperature")]
    DebugSensorsTemperature,
    #[strum(serialize = "debug/sensors/humidity")]
    DebugSensorsHumidity,
    #[strum(serialize = "debug/sensors/pressure")]
    DebugSensorsPressure,
    #[strum(serialize = "debug/sensors/mag")]
    DebugSensorsMag,
    #[strum(serialize = "speed")]
    PilotDisplaySpeed,
    #[strum(serialize = "altitude")]
    PilotDisplayAltitude,
    #[strum(serialize = "heading")]
    PilotDisplayHeading,
    #[strum(serialize = "closest_line")]
    PilotDisplayClosestLine,
}

/// Debug packet
#[derive(Deserialize)]
pub struct DebugSensorPacket {
    value: u32,
}

impl KnownTopic {
    pub(crate) fn parse_packet(self, p: &str) -> Result<MqttPoint, serde_json::Error> {
        match self {
            Self::DebugSensorsTemperature
            | Self::DebugSensorsHumidity
            | Self::DebugSensorsPressure
            | Self::DebugSensorsMag => {
                let sp: DebugSensorPacket = serde_json::from_str(p)?;
                Ok(self.into_mqtt_point(sp.value.into()))
            }
            Self::PilotDisplaySpeed => {
                let p = serde_json::from_str::<PilotDisplaySpeedPacket>(p)?;
                let value = p
                    .speed
                    .parse()
                    .expect("Failed to parse PilotDisplaySpeedPacket");
                Ok(self.into_mqtt_point(value))
            }
            Self::PilotDisplayAltitude => {
                let p: PilotDisplayAltitudePacket = serde_json::from_str(p)?;
                let value = p
                    .height
                    .parse()
                    .expect("Failed to parse PilotDisplayAltitudePacket");
                Ok(self.into_mqtt_point(value))
            }
            Self::PilotDisplayHeading => {
                let p: PilotDisplayHeadingPacket = serde_json::from_str(p)?;
                let value = p
                    .heading
                    .parse()
                    .expect("Failed to parse PilotDisplayHeadingPacket");
                Ok(self.into_mqtt_point(value))
            }
            Self::PilotDisplayClosestLine => {
                let p: PilotDisplayClosestLinePacket = serde_json::from_str(p)?;
                Ok(self.into_mqtt_point(p.distance))
            }
        }
    }

    fn into_mqtt_point(self, value: f64) -> MqttPoint {
        MqttPoint::new(self.to_string(), value)
    }
}

#[derive(Deserialize)]
pub struct PilotDisplaySpeedPacket {
    #[serde(rename(deserialize = "Speed"))]
    speed: String,
}

#[derive(Deserialize)]
pub struct PilotDisplayAltitudePacket {
    #[serde(rename(deserialize = "Height"))]
    height: String,
}
#[derive(Deserialize)]
pub struct PilotDisplayHeadingPacket {
    #[serde(rename(deserialize = "Heading"))]
    heading: String,
}

#[derive(Deserialize)]
pub struct PilotDisplayClosestLinePacket {
    distance: f64,
}

pub(crate) fn parse_packet(topic: &str, payload: &str) -> Option<MqttPoint> {
    if let Ok(known) = KnownTopic::from_str(topic) {
        match known.parse_packet(payload) {
            Ok(mp) => Some(mp),
            Err(e) => {
                log::error!("{e}");
                debug_assert!(false, "{e}");
                None
            }
        }
    } else {
        log::warn!("Unknown topic: {topic}, attempting to parse as f64");
        parse_unknown_topic(topic, payload)
    }
}

fn parse_unknown_topic(topic: &str, payload: &str) -> Option<MqttPoint> {
    let now = now_timestamp();
    match payload.parse::<f64>() {
        Ok(num) => {
            let point = PlotPoint::new(now, num);
            let mqtt_data = MqttPoint {
                topic: topic.to_owned(),
                point,
            };
            Some(mqtt_data)
        }
        Err(e) => {
            log::error!("Payload parse error: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_enum_parse_packet() {
        let s = "debug/sensors/temperature".to_owned();
        let e = KnownTopic::from_str(&s).unwrap();
        let payload = r#"{ "value": 40 }"#;
        let p = e.parse_packet(payload).unwrap();
        dbg!(p);
    }

    #[test]
    fn test_parse_pilot_display_speed_packet() {
        let s = "speed".to_owned();
        let e = KnownTopic::from_str(&s).unwrap();
        let payload = json!({
            "Speed": "12.433",
        })
        .to_string();
        let p = e.parse_packet(&payload).unwrap();
        dbg!(p);
    }

    #[test]
    fn test_parse_pilot_display_closest_line() {
        let t = "closest_line";
        let known_topic = KnownTopic::from_str(t).unwrap();
        let payload = r#"{"id": 12, "flight_line": "L501100", "distance": 1.84167211, "mode": "automatic", "filename": "20231023_Bremervoerde_Combined_300_NS_32N.kml"}"#;
        let p = known_topic.parse_packet(payload).unwrap();
        dbg!(p);
    }
}
