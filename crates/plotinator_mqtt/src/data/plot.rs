use egui_plot::PlotPoint;

use super::listener::{MqttData, MqttTopicData, MqttTopicDataWrapper, TopicPayload};

/// A collection of accumulated plot points from various MQTT topics
///
/// This is the basis for all line plots from MQTT data
#[derive(Default)]
pub(crate) struct MqttPlotData {
    mqtt_plot_data: Vec<MqttPlotPoints>,
}

impl MqttPlotData {
    fn insert_inner_data(&mut self, data: MqttTopicData) {
        if let Some(mp) = self
            .mqtt_plot_data
            .iter_mut()
            .find(|mp| mp.topic == data.topic())
        {
            match data.payload {
                TopicPayload::Point(plot_point) => mp.data.push(plot_point),
                TopicPayload::Points(mut plot_points) => mp.data.append(&mut plot_points),
            }
        } else {
            self.mqtt_plot_data.push(data.into());
        }
    }

    pub(crate) fn insert_data(&mut self, data: MqttData) {
        match data.inner {
            MqttTopicDataWrapper::Topic(mqtt_topic_data) => self.insert_inner_data(mqtt_topic_data),
            MqttTopicDataWrapper::Topics(mqtt_topic_data_vec) => {
                for mtd in mqtt_topic_data_vec {
                    self.insert_inner_data(mtd);
                }
            }
        }
    }

    pub(crate) fn plots(&self) -> &[MqttPlotPoints] {
        &self.mqtt_plot_data
    }
}

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
