use std::sync::{Arc, atomic::AtomicBool, mpsc::Receiver};

use crate::{
    MqttPlotPoints,
    client::MqttClient,
    data::{listener::MqttData, plot::MqttPlotData},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ConnectionState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub(crate) enum MqttMessage {
    ConnectionState(ConnectionState),
    Data(MqttData),
}

pub fn spawn_mqtt_listener(
    stop_flag: &mut Arc<AtomicBool>,
    broker_host: String,
    broker_port: u16,
    topics: &[String],
) -> MqttDataReceiver {
    let (tx, rx) = std::sync::mpsc::channel();
    let client = MqttClient::new(
        Arc::clone(stop_flag),
        broker_host,
        broker_port,
        topics.to_owned(),
        tx.clone(),
    );
    client.spawn();
    MqttDataReceiver::new(rx, topics.to_owned())
}

pub struct MqttDataReceiver {
    subscribed_topics: Vec<String>,
    mqtt_plot_data: MqttPlotData,
    recv: Receiver<MqttMessage>,
    state: ConnectionState,
}

impl MqttDataReceiver {
    pub(crate) fn new(recv: Receiver<MqttMessage>, subscribed_topics: Vec<String>) -> Self {
        Self {
            subscribed_topics,
            mqtt_plot_data: MqttPlotData::default(),
            recv,
            state: ConnectionState::Disconnected,
        }
    }

    /// Returns true if the listener is connected to the MQTT broker
    pub fn connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    pub fn plots(&self) -> &[MqttPlotPoints] {
        self.mqtt_plot_data.plots()
    }

    pub fn poll(&mut self) {
        while let Ok(mqtt_msg) = self.recv.try_recv() {
            log::debug!("Got MQTT Message: {mqtt_msg:?}");
            match mqtt_msg {
                MqttMessage::ConnectionState(connection_state) => self.state = connection_state,
                MqttMessage::Data(mqtt_data) => self.mqtt_plot_data.insert_data(mqtt_data),
            }
        }
    }

    pub fn subscribed_topics(&self) -> &[String] {
        &self.subscribed_topics
    }
}
