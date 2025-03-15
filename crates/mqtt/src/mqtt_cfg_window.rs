use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Receiver,
        Arc,
    },
};

use crate::{
    broker_validator::BrokerValidator, data_receiver::MqttDataReceiver,
    topic_discoverer::TopicDiscoverer, MqttPoint,
};

pub struct MqttConfigWindow {
    broker_ip: String,
    broker_port: String,
    text_input_topic: String,
    selected_topics: Vec<String>,

    broker_validator: BrokerValidator,
    topic_discoverer: TopicDiscoverer,

    mqtt_stop_flag: Arc<AtomicBool>,
}

impl MqttConfigWindow {
    /// Returns a reference to the selected topics of this [`MqttConfigWindow`].
    pub fn selected_topics(&self) -> &[String] {
        &self.selected_topics
    }

    pub fn selected_topics_as_mut(&mut self) -> &mut [String] {
        &mut self.selected_topics
    }

    pub fn remove_empty_selected_topics(&mut self) {
        self.selected_topics.retain(|t| !t.is_empty());
    }

    /// Adds `topic`` to the selected topics collection if it is not empty and the collection doesn't already contain it
    pub fn add_selected_topic(&mut self, topic: String) {
        if !topic.is_empty() && !self.selected_topics.contains(&topic) {
            self.selected_topics.push(topic);
        }
    }

    /// Returns the add text input topic of this [`MqttConfigWindow`].
    pub fn add_text_input_topic(&mut self) {
        self.add_selected_topic(self.text_input_topic.to_owned());
        self.text_input_topic.clear();
    }

    /// Returns a mutable reference to the text input topic of this [`MqttConfigWindow`].
    pub fn text_input_topic_as_mut(&mut self) -> &mut String {
        &mut self.text_input_topic
    }

    /// Returns a mutable reference to the broker ip of this [`MqttConfigWindow`].
    pub fn broker_ip_as_mut(&mut self) -> &mut String {
        &mut self.broker_ip
    }

    /// Returns a reference to the broker ip of this [`MqttConfigWindow`].
    pub fn broker_ip(&self) -> &str {
        &self.broker_ip
    }

    pub fn broker_port_as_mut(&mut self) -> &mut String {
        &mut self.broker_port
    }

    /// Sets the stop flag to stop the MQTT client
    pub fn set_stop_flag(&mut self) {
        self.mqtt_stop_flag.store(true, Ordering::SeqCst);
    }

    pub fn get_stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.mqtt_stop_flag)
    }

    pub fn reset_stop_flag(&mut self) {
        self.mqtt_stop_flag.store(false, Ordering::SeqCst);
    }

    pub fn stop_topic_discovery(&mut self) {
        self.topic_discoverer.stop();
    }

    pub fn discovery_active(&self) -> bool {
        self.topic_discoverer.active()
    }

    pub fn poll_discovered_topics(&mut self) -> Result<(), String> {
        self.topic_discoverer.poll_discovered_topics()
    }

    pub fn start_topic_discovery(&mut self) {
        if let Ok(port) = self.broker_port.parse::<u16>() {
            self.topic_discoverer.start(self.broker_ip.clone(), port);
        }
    }

    pub fn discovered_topics(&self) -> &HashSet<String> {
        &self.topic_discoverer.discovered_topics()
    }

    pub fn discovered_topics_sorted(&self) -> Vec<String> {
        self.topic_discoverer.discovered_topics_sorted()
    }

    pub fn broker_status(&self) -> Option<&Result<(), String>> {
        self.broker_validator.broker_status()
    }

    pub fn validation_in_progress(&self) -> bool {
        self.broker_validator.validation_in_progress()
    }

    pub fn poll_broker_status(&mut self) {
        self.broker_validator
            .poll_broker_status(&self.broker_ip, &self.broker_port);
    }

    pub fn spawn_mqtt_listener(&mut self) -> MqttDataReceiver {
        self.reset_stop_flag();
        let broker = self.broker_ip().to_owned();
        let topics = self.selected_topics().to_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        let thread_stop_flag = self.get_stop_flag();
        std::thread::Builder::new()
            .name("mqtt-listener".into())
            .spawn(move || {
                crate::mqtt_listener(tx, broker, topics, thread_stop_flag);
            })
            .expect("Failed spawning MQTT listener thread");
        MqttDataReceiver::new(rx)
    }
}

impl Default for MqttConfigWindow {
    fn default() -> Self {
        Self {
            broker_ip: "127.0.0.1".into(),
            broker_port: "1883".into(),
            selected_topics: Default::default(),
            text_input_topic: Default::default(),

            broker_validator: BrokerValidator::default(),
            topic_discoverer: TopicDiscoverer::default(),

            mqtt_stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}
