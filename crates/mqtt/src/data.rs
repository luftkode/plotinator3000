use egui_plot::PlotPoint;

use crate::known_topics::now_timestamp;

/// Accumulated plot points from an MQTT topic
#[derive(Debug)]
pub struct MqttPoints {
    pub topic: String,
    pub data: Vec<PlotPoint>,
}

/// A single plot point with its topic origin
#[derive(Debug)]
pub struct MqttPoint {
    pub topic: String,
    pub point: PlotPoint,
}

impl MqttPoint {
    pub fn new(topic: String, value: f64) -> Self {
        Self {
            topic,
            point: PlotPoint {
                x: now_timestamp(),
                y: value,
            },
        }
    }
}
