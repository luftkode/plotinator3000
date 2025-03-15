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

pub(crate) mod broker_validator;
pub mod util;

pub(crate) struct TopicDiscoverer {}

pub struct MqttConfigWindow {
    pub broker_ip: String,
    pub broker_port: String,
    pub topics: Vec<String>,
    pub new_topic: String,

    broker_validator: BrokerValidator,

    /// Topic discovery fields
    discovery_active: bool,
    pub discovered_topics: HashSet<String>, // Use HashSet for deduplication
    discovery_rx: Option<mpsc::Receiver<String>>,
    stop_discovery_flag: Arc<AtomicBool>,

    /// UI state
    pub discovery_handle: Option<std::thread::JoinHandle<()>>,
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
        self.stop_discovery_flag.store(true, Ordering::SeqCst);
        self.discovery_active = false;
    }

    pub fn get_stop_discovery_flag(&mut self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_discovery_flag)
    }

    pub fn reset_stop_discovery_flag(&mut self) {
        self.discovered_topics.clear();
        self.stop_discovery_flag.store(false, Ordering::SeqCst);
    }

    pub fn discovery_active(&self) -> bool {
        self.discovery_active
    }

    pub fn poll_discovered_topics(&mut self) -> Result<(), String> {
        if let Some(rx) = &mut self.discovery_rx {
            while let Ok(topic) = rx.try_recv() {
                if topic.starts_with("!ERROR: ") {
                    return Err(topic[8..].to_owned());
                } else {
                    self.discovered_topics.insert(topic);
                }
            }
        }
        Ok(())
    }

    pub fn start_topic_discovery(&mut self) {
        if let Ok(port_u16) = self.broker_port.parse::<u16>() {
            self.reset_stop_discovery_flag();
            self.discovery_active = true;

            let host = self.broker_ip.clone();
            let (tx, rx) = mpsc::channel();

            self.discovery_rx = Some(rx);
            self.discovery_handle = Some(start_discovery(
                host,
                port_u16,
                self.get_stop_discovery_flag(),
                tx,
            ));
        }
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
            topics: Default::default(),
            new_topic: Default::default(),

            broker_validator: BrokerValidator::default(),

            discovery_active: false,
            discovered_topics: Default::default(),
            discovery_rx: None,
            stop_discovery_flag: Default::default(),

            discovery_handle: None,
            mqtt_stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub fn start_discovery(
    host: String,
    port: u16,
    stop_flag: Arc<AtomicBool>,
    tx: mpsc::Sender<String>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("Time went backwards");
        let client_id = format!("discover-{}", timestamp.as_millis());

        log::info!("Subscribing for discovery with id={client_id}, broker address={host}:{port}");
        let mut mqttoptions = MqttOptions::new(client_id, host, port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqttoptions, 100);

        if let Err(e) = client.subscribe("#", rumqttc::QoS::AtMostOnce) {
            log::error!("Subscribe err={e}");
            let _ = tx.send(format!("!ERROR: {}", e));
            return;
        }

        for notification in connection.iter() {
            if stop_flag.load(Ordering::Relaxed) {
                log::info!("Stopping discovery!");
                break;
            }

            match notification {
                Ok(event) => {
                    if let Event::Incoming(Packet::Publish(p)) = event {
                        log::info!("Discovered topic={}", p.topic);
                        let _ = tx.send(p.topic);
                    }
                }
                Err(e) => {
                    log::error!("Discover connection err={e}");
                    let _ = tx.send(format!("!ERROR: {}", e));
                    break;
                }
            }
        }
        let _ = client.disconnect();
    })
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
