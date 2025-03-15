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
    pub broker_ip: String,
    pub broker_port: String,
    pub selected_topics: Vec<String>,
    pub new_topic: String,

    broker_validator: BrokerValidator,
    topic_discoverer: TopicDiscoverer,

    mqtt_stop_flag: Arc<AtomicBool>,
}

impl MqttConfigWindow {
    pub fn set_stop_flag(&mut self) {
        self.mqtt_stop_flag.store(true, Ordering::SeqCst);
    }

    pub fn get_stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.mqtt_stop_flag)
    }

    pub fn reset_stop_flag(&mut self) {
        self.mqtt_stop_flag.store(false, Ordering::SeqCst);
    }

    pub fn set_stop_discovery_flag(&mut self) {
        self.topic_discoverer.set_stop_flag();
    }

    pub fn get_stop_discovery_flag(&mut self) -> Arc<AtomicBool> {
        self.topic_discoverer.get_stop_flag()
    }

    pub fn reset_stop_discovery_flag(&mut self) {
        self.topic_discoverer.reset_stop_flag();
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
            new_topic: Default::default(),

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

pub fn mqtt_receiver(
    tx: mpsc::Sender<MqttPoint>,
    broker: String,
    topics: Vec<String>,
    stop_flag: Arc<AtomicBool>,
) {
    let mut mqttoptions = MqttOptions::new("plotinator3000", broker, 1883);
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
