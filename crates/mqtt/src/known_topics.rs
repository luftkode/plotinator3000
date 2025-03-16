use std::str::FromStr;

use egui_plot::PlotPoint;
use serde::Deserialize;
use strum_macros::{Display, EnumString};

use crate::MqttPoint;

fn now_timestamp() -> f64 {
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
    #[strum(serialize = "Speed")]
    PilotDisplaySpeed,
}

/// Debug packet
#[derive(Deserialize)]
pub struct DebugSensorPacket {
    value: u32,
}

impl KnownTopic {
    pub(crate) fn parse_packet(&self, p: &str) -> Result<MqttPoint, serde_json::Error> {
        match self {
            Self::DebugSensorsTemperature
            | Self::DebugSensorsHumidity
            | Self::DebugSensorsPressure
            | Self::DebugSensorsMag => {
                let sp: DebugSensorPacket = serde_json::from_str(p)?;

                Ok(MqttPoint {
                    topic: self.to_string(),
                    point: PlotPoint::new(now_timestamp(), sp.value),
                })
            }
            Self::PilotDisplaySpeed => {
                let p = serde_json::from_str::<PilotDisplaySpeedPacket>(p)?;
                Ok(MqttPoint {
                    topic: self.to_string(),
                    point: PlotPoint::new(now_timestamp(), p.speed),
                })
            }
        }
    }
}

#[derive(Deserialize)]
pub struct PilotDisplaySpeedPacket {
    speed: f64,
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos() as f64;
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let s = "Speed".to_owned();
        let e = KnownTopic::from_str(&s).unwrap();
        let payload = r#"{ "speed": 12.433 }"#;
        let p = e.parse_packet(payload).unwrap();
        dbg!(p);
    }
}
