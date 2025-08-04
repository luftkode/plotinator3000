use egui_plot::PlotPoint;
use pilot_display::{PilotDisplayCoordinates, PilotDisplayRemainingDistance};
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
    #[strum(serialize = "dt/blackbird/sky-ubx/x/coordinates")]
    PilotDisplayCoordinates,
    #[strum(serialize = "dt/blackbird/pd-backend/remaining-distance")]
    PilotDisplayRemainingDistance,
    #[strum(serialize = "$SYS/broker/uptime")]
    SYSBrokerUptime,
    // We cannot meaningfully plot this, but we use it to show the version when choosing a broker to connect to
    #[strum(serialize = "$SYS/broker/version")]
    SYSBrokerVersion,
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
            Self::PilotDisplayCoordinates => {
                let p: PilotDisplayCoordinates = serde_json::from_str(p)?;
                let lat = MqttTopicData::single(self.subtopic_str("lat"), p.lat());
                let lon = MqttTopicData::single(self.subtopic_str("lon"), p.lon());
                let data = MqttData::multiple(vec![lat, lon]);
                Ok(Some(data))
            }
            Self::PilotDisplayRemainingDistance => {
                let p: PilotDisplayRemainingDistance = serde_json::from_str(p)?;
                Ok(Some(self.into_single_mqtt_data(p.distance())))
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
    use std::str::FromStr as _;

    use super::*;
    use plotinator_test_util::*;
    use serde_json::json;

    #[test]
    fn test_enum_parse_packet() -> TestResult {
        let known_topic = KnownTopic::from_str("debug/sensors/temperature")?;
        let payload = r#"{ "value": 40 }"#;
        let mqtt_data = known_topic.parse_packet(payload)?;
        dbg!(&mqtt_data);
        assert!(mqtt_data.is_some());
        Ok(())
    }

    #[test]
    fn test_parse_pilot_display_coordinate_packet() -> TestResult {
        let known_topic = KnownTopic::from_str("dt/blackbird/sky-ubx/x/coordinates")?;
        let payload = json!({
            "lon": 10.1473,"lat": 56.2179
        })
        .to_string();
        let mqtt_data = known_topic.parse_packet(&payload)?;
        dbg!(&mqtt_data);
        assert!(mqtt_data.is_some());
        Ok(())
    }

    #[test]
    fn test_parse_pilot_display_remaining_distance_packet() -> TestResult {
        let known_topic = KnownTopic::from_str("dt/blackbird/pd-backend/remaining-distance")?;
        let payload = json!({
            "distance": 1560.514601457434
        })
        .to_string();
        let mqtt_data = known_topic.parse_packet(&payload)?;
        dbg!(&mqtt_data);
        assert!(mqtt_data.is_some());
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
