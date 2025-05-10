use egui_plot::PlotPoint;
use pilot_display::{
    PilotDisplayAltitudePacket, PilotDisplayClosestLinePacket, PilotDisplayHeadingPacket,
    PilotDisplaySpeedPacket,
};
use serde::Deserialize;
use strum_macros::{Display, EnumString};

use crate::{
    data::listener::{MqttData, MqttTopicData},
    util,
};

pub(crate) mod pilot_display;

/// Known topics can have custom payloads that have an associated known packet structure
/// which allows recognizing and parsing them appropriately
#[derive(EnumString, Display)]
pub(crate) enum KnownTopic {
    #[strum(serialize = "debug/sensors/temperature")]
    DebugSensorsTemperature,
    #[strum(serialize = "debug/sensors/humidity")]
    DebugSensorsHumidity,
    #[strum(serialize = "debug/sensors/pressure")]
    DebugSensorsPressure,
    #[strum(serialize = "debug/sensors/mag")]
    DebugSensorsMag,
    #[strum(serialize = "debug/sensors/gps")]
    DebugSensorsGps,
    #[strum(serialize = "speed")]
    PilotDisplaySpeed,
    #[strum(serialize = "altitude")]
    PilotDisplayAltitude,
    #[strum(serialize = "heading")]
    PilotDisplayHeading,
    #[strum(serialize = "closest_line")]
    PilotDisplayClosestLine,
    #[strum(serialize = "$SYS/broker/uptime")]
    SYSBrokerUptime,
    // We cannot meaningfully plot this, but we use it to show the version when choosing a broker to connect to
    #[strum(serialize = "$SYS/broker/version")]
    SYSBrokerVersion
}

/// Debug packet with a single value
/// e.g. { "value": 70.56164 }
#[derive(Deserialize)]
pub(crate) struct DebugSensorPacket {
    value: f64,
}

/// A pair of values and timestamp
/// e.g. {"value": 3167.38561, "timestamp": "1742662585.527223099"}
#[derive(Deserialize)]
pub(crate) struct ValueWithTimestampString {
    value: f64,
    timestamp: String, // Format determined by `data +%s.%N`
}

/// Debug packet with multiple values
/// e.g. { "value1": 44.50188, "value2": 41.58077 }
#[derive(Deserialize)]
pub(crate) struct DebugSensorsGps {
    value1: f64,
    value2: f64,
}

impl KnownTopic {
    pub(crate) fn parse_packet(self, p: &str) -> anyhow::Result<Option<MqttData>> {
        match self {
            Self::PilotDisplaySpeed => {
                let p = serde_json::from_str::<PilotDisplaySpeedPacket>(p)?;
                let value = p.speed()?;
                Ok(Some(self.into_single_mqtt_data(value)))
            }
            Self::PilotDisplayAltitude => {
                let p: PilotDisplayAltitudePacket = serde_json::from_str(p)?;
                let value = p.height()?;
                Ok(Some(self.into_single_mqtt_data(value)))
            }
            Self::PilotDisplayHeading => {
                let p: PilotDisplayHeadingPacket = serde_json::from_str(p)?;
                let value = p.heading()?;
                Ok(Some(self.into_single_mqtt_data(value)))
            }
            Self::PilotDisplayClosestLine => {
                let p: PilotDisplayClosestLinePacket = serde_json::from_str(p)?;
                let distance = p.distance();
                Ok(Some(self.into_single_mqtt_data(distance)))
            }
            // Debug topics for development and for inspiration for how to implement parsing of various kinds of
            // topics and payloads
            Self::DebugSensorsTemperature
            | Self::DebugSensorsHumidity
            | Self::DebugSensorsPressure => {
                let sp: DebugSensorPacket = serde_json::from_str(p)?;
                Ok(Some(self.into_single_mqtt_data(sp.value)))
            }
            Self::DebugSensorsGps => {
                let sp: DebugSensorsGps = serde_json::from_str(p)?;
                let td1 = MqttTopicData::single(self.subtopic_str("lat"), sp.value1);
                let td2 = MqttTopicData::single(self.subtopic_str("lon"), sp.value2);
                let d = MqttData::multiple(vec![td1, td2]);
                Ok(Some(d))
            }
            Self::DebugSensorsMag => {
                let values: Vec<ValueWithTimestampString> =
                    serde_json::from_str(p).expect("Debug failure");
                let mut points: Vec<PlotPoint> = vec![];
                for v in values {
                    let t = util::parse_timestamp_to_nanos_f64(&v.timestamp)
                        .expect("failed parsing timestamp");
                    let p = PlotPoint::new(t, v.value);
                    points.push(p);
                }
                let td = MqttTopicData::multiple(self.to_string(), points);
                Ok(Some(MqttData::single(td)))
            }
            Self::SYSBrokerUptime => {
                // Example payload: '2256144 seconds'
                let uptime_str = p.trim_end_matches(" seconds");
                let uptime: f64 = uptime_str.parse()?;
                Ok(Some(self.into_single_mqtt_data(uptime)))
            }
            Self::SYSBrokerVersion => {
                // Does not make sense to plot
                Ok(None)
            }
        }
    }

    // Converts a value to a simple MqttData with just a single topic and point
    // this is appropriate for very simple topics with just a single point per message
    fn into_single_mqtt_data(self, value: f64) -> MqttData {
        let topic_data = MqttTopicData::single(self.to_string(), value);
        MqttData::single(topic_data)
    }

    // Returns a string with the topic name appended with [subtopic_name]
    //
    // helper function to construct topic strings with a 'subvalue' specifier
    // for topics that essentially have subtopics, meaning they receive payload
    //  with multiple different kind of values, e.g. a gps
    // that publishes both longitude and latitude
    fn subtopic_str(&self, subtopic_name: &str) -> String {
        let mut topic_with_subvalue = self.to_string();
        // yes this could be a single line of format!("[{subtopic_name}]")
        // but this potentially has better performance
        topic_with_subvalue.push('[');
        topic_with_subvalue.push_str(subtopic_name);
        topic_with_subvalue.push(']');
        topic_with_subvalue
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use serde_json::json;
    use test_util::*;

    #[test]
    fn test_enum_parse_packet() -> TestResult {
        let known_topic = KnownTopic::from_str("debug/sensors/temperature")?;
        let payload = r#"{ "value": 40 }"#;
        let mqtt_data = known_topic.parse_packet(payload)?;
        dbg!(mqtt_data);
        Ok(())
    }

    #[test]
    fn test_parse_pilot_display_speed_packet() -> TestResult {
        let known_topic = KnownTopic::from_str("speed")?;
        let payload = json!({
            "Speed": "12.433",
        })
        .to_string();
        let mqtt_data = known_topic.parse_packet(&payload)?;
        dbg!(mqtt_data);
        Ok(())
    }

    #[test]
    fn test_parse_pilot_display_closest_line() -> TestResult {
        let known_topic = KnownTopic::from_str("closest_line")?;
        let payload = r#"{"id": 12, "flight_line": "L501100", "distance": 1.84167211, "mode": "automatic", "filename": "20231023_Bremervoerde_Combined_300_NS_32N.kml"}"#;
        let mqtt_data = known_topic.parse_packet(payload)?;
        dbg!(mqtt_data);
        Ok(())
    }

    #[test]
    fn test_parse_debug_buffered_packet() -> TestResult {
        let topic = KnownTopic::DebugSensorsMag;
        let payload = r#"[{"value": 4125.26202, "timestamp": "1742661659.597146009"},{"value": 5319.64538, "timestamp": "1742661659.597977050"},{"value": 3088.24687, "timestamp": "1742661659.598809170"},{"value": 3032.34963, "timestamp": "1742661659.599677984"},{"value": 3220.23746, "timestamp": "1742661659.600710924"}]"#;
        let _decoded: Vec<ValueWithTimestampString> = serde_json::from_str(payload)?;
        let known_topic = KnownTopic::from_str(&topic.to_string())?;
        let mqtt_data = known_topic.parse_packet(payload)?;
        dbg!(mqtt_data);
        Ok(())
    }
}
