use broker_validator::BrokerValidator;
use egui_plot::PlotPoint;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
use topic_discoverer::TopicDiscoverer;

pub(crate) mod broker_validator;
pub(crate) mod topic_discoverer;
pub mod util;

pub struct MqttConfigWindow {
    broker_ip: String,
    broker_port: String,
    text_input_topic: String,
    pub selected_topics: Vec<String>,

    broker_validator: BrokerValidator,
    topic_discoverer: TopicDiscoverer,

    mqtt_stop_flag: Arc<AtomicBool>,
}

impl MqttConfigWindow {
    /// Returns a reference to the selected topics of this [`MqttConfigWindow`].
    pub fn selected_topics(&self) -> &[String] {
        &self.selected_topics
    }

    /// Returns a mutable reference to the selected topics of this [`MqttConfigWindow`].
    pub fn selected_topics_as_mut(&mut self) -> &mut Vec<String> {
        &mut self.selected_topics
    }

    /// Adds `topic`` to the selected topics collection if it is not empty and the collection doesn't already contain it
    pub fn add_selected_topic(&mut self, topic: String) {
        if !topic.is_empty() && !self.selected_topics.contains(&topic) {
            self.selected_topics.push(topic);
        }
    }

    /// Returns the add text input topic of this [`MqttConfigWindow`].
    pub fn add_text_input_topic(&mut self) {
        self.add_selected_topic(self.text_input_topic().to_owned());
        self.text_input_topic.clear();
    }

    /// Returns a mutable reference to the text input topic of this [`MqttConfigWindow`].
    pub fn text_input_topic_as_mut(&mut self) -> &mut String {
        &mut self.text_input_topic
    }

    /// Returns a reference to the text input topic of this [`MqttConfigWindow`].
    pub fn text_input_topic(&self) -> &str {
        &self.text_input_topic
    }

    /// Returns a mutable reference to the broker ip of this [`MqttConfigWindow`].
    pub fn broker_ip_as_mut(&mut self) -> &mut String {
        &mut self.broker_ip
    }

    /// Returns a reference to the broker ip of this [`MqttConfigWindow`].
    pub fn broker_ip(&self) -> &str {
        &self.broker_ip
    }

    /// Returns a mutable reference to the broker port of this [`MqttConfigWindow`].
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

#[derive(Debug)]
pub struct MqttData {
    pub topic: String,
    pub data: Vec<PlotPoint>,
}

#[derive(Debug)]
pub struct MqttPoint {
    pub topic: String,
    pub point: PlotPoint,
}

pub fn mqtt_listener(
    tx: mpsc::Sender<MqttPoint>,
    broker: String,
    topics: Vec<String>,
    stop_flag: Arc<AtomicBool>,
) {
    let timestamp_id = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    let mut mqttoptions = MqttOptions::new(format!("plotinator3000-{timestamp_id}"), broker, 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut connection) = Client::new(mqttoptions, 10);
    for t in topics {
        if let Err(e) = client.subscribe(t, QoS::AtMostOnce) {
            log::error!("Subscribe error: {e}");
        }
    }

    // Iterate to poll the eventloop for connection progress
    for notification in connection.iter() {
        if stop_flag.load(Ordering::Relaxed) {
            log::info!("Stopping!");
            break;
        }
        match notification {
            Ok(event) => {
                if let Event::Incoming(Packet::Publish(publish)) = event {
                    let topic = publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    log::info!("Received on topic={topic}, payload={payload}");
                    match payload.parse::<f64>() {
                        Ok(num) => {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos() as f64;
                            log::info!("now={now}");
                            let point = PlotPoint::new(now, num);
                            let mqtt_data = MqttPoint { topic, point };
                            if let Err(e) = tx.send(mqtt_data) {
                                log::error!("Send err={e}");
                            }
                        }
                        Err(e) => log::error!("Payload parse error: {e}"),
                    }
                }
            }

            Err(e) => log::error!("{e}"),
        }
    }
    client.disconnect().unwrap();
}
