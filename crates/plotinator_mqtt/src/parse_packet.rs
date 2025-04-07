use std::str::FromStr as _;

use known_topic::KnownTopic;
use crate::data::listener::{MqttData, MqttTopicData};

pub mod known_topic;

pub(crate) fn parse_packet(topic: &str, payload: &str) -> Option<MqttData> {
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

fn parse_unknown_topic(topic: &str, payload: &str) -> Option<MqttData> {
    match payload.parse::<f64>() {
        Ok(num) => {
            let md = MqttData::single(MqttTopicData::single(topic.to_owned(), num));
            Some(md)
        }
        Err(e) => {
            log::error!("Failed to parse payload of topic '{topic}': {e}");
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
