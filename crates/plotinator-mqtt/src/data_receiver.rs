use std::sync::{Arc, atomic::AtomicBool, mpsc::Receiver};

use crate::{
    MqttPlotPoints,
    data::{listener::MqttData, plot::MqttPlotData},
};

pub fn spawn_mqtt_listener(
    stop_flag: &mut Arc<AtomicBool>,
    broker_host: String,
    broker_port: String,
    topics: &[String],
) -> MqttDataReceiver {
    let stop_flag_clone = Arc::clone(stop_flag);
    let (tx, rx) = std::sync::mpsc::channel();
    let topics_clone = topics.to_owned();
    std::thread::Builder::new()
        .name("mqtt-listener".into())
        .spawn(move || {
            crate::mqtt_listener::mqtt_listener(
                &tx,
                broker_host,
                broker_port.parse().expect("invalid broker port"),
                topics_clone,
                &stop_flag_clone,
            );
        })
        .expect("Failed spawning MQTT listener thread");
    MqttDataReceiver::new(rx, topics.to_owned())
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
