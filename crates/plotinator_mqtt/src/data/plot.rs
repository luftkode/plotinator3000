use egui_plot::PlotPoint;

use super::listener::{MqttTopicData, TopicPayload};

/// Accumulated plot points from an MQTT topic
///
/// This is the final state of received MQTT data
/// where it is plotable
#[derive(Debug)]
pub struct MqttPlotPoints {
    pub topic: String,
    pub data: Vec<PlotPoint>,
}

impl From<MqttTopicData> for MqttPlotPoints {
    fn from(value: MqttTopicData) -> Self {
        let data = match value.payload {
            TopicPayload::Point(plot_point) => vec![plot_point],
            TopicPayload::Points(plot_points) => plot_points,
        };
        Self {
            topic: value.topic,
            data,
        }
    }
}
