use std::sync::mpsc::Receiver;

use crate::{
    data::{listener::MqttData, plot::MqttPlotData},
    MqttPlotPoints,
};

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
        self.mqtt_plot_data.plots()
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
