use std::sync::mpsc::Receiver;

use crate::{
    data::listener::{MqttData, MqttTopicData, MqttTopicDataWrapper, TopicPayload},
    MqttPlotPoints,
};

#[derive(Default)]
pub struct MqttPlotData {
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
}

pub struct MqttDataReceiver {
    subscribed_topics: Vec<String>,
    mqtt_plot_data: MqttPlotData,
    recv: Receiver<MqttData>,
}

impl MqttDataReceiver {
    pub(crate) fn new(recv: Receiver<MqttData>, subscribed_topics: Vec<String>) -> Self {
        Self {
            subscribed_topics,
            mqtt_plot_data: MqttPlotData::default(),
            recv,
        }
    }

    pub fn plots(&self) -> &[MqttPlotPoints] {
        &self.mqtt_plot_data.mqtt_plot_data
    }

    pub fn poll(&mut self) {
        while let Ok(mqtt_data) = self.recv.try_recv() {
            log::debug!("Got MqttData: {mqtt_data:?}");
            self.mqtt_plot_data.insert_data(mqtt_data);
        }
    }

    pub fn subscribed_topics(&self) -> &[String] {
        &self.subscribed_topics
    }
}
