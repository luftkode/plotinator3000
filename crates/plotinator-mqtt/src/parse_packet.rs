use std::str::FromStr as _;

use crate::data::listener::{MqttData, MqttTopicData};
use known_topic::KnownTopic;

pub mod known_topic;

#[inline]
pub(crate) fn parse_packet(topic: &str, payload: &str) -> Option<MqttData> {
    if let Ok(known) = KnownTopic::from_str(topic) {
        match known.parse_packet(payload) {
            Ok(mp) => mp,
            Err(e) => {
                log::error!("{e}");
                debug_assert!(false, "{e}");
                None
            }
        }
    } else {
        log::debug!("Unknown topic: {topic}, attempting to parse as f64");
        parse_unknown_topic(topic, payload)
    }
}

#[inline]
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

    #[test]
    fn test_enum_parse_packet() {
        let s = "debug/sensors/temperature".to_owned();
        let e = KnownTopic::from_str(&s).unwrap();
        let payload = r#"{ "value": 40 }"#;
        let p = e.parse_packet(payload).unwrap();
        dbg!(p);
    }
}
