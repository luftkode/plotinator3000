use egui_plot::PlotPoint;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

pub mod util;

pub struct MqttConfigWindow {
    pub broker_ip: String,
    pub broker_port: String,
    pub topics: Vec<String>,
    pub new_topic: String,
    pub open: bool,

    /// Broker discovery fields
    pub previous_broker_input: String,
    pub broker_status: Option<Result<(), String>>,
    pub validation_in_progress: bool,
    pub last_input_change: Option<Instant>,

    /// Topic discovery fields
    pub discovery_active: bool,
    pub discovered_topics: HashSet<String>, // Use HashSet for deduplication
    pub discovery_rx: Option<mpsc::Receiver<String>>,
    pub discovery_stop: Arc<AtomicBool>,

    /// UI state
    pub broker_validation_receiver: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    pub discovery_handle: Option<std::thread::JoinHandle<()>>,
    pub mqtt_stop_flag: Arc<AtomicBool>,
}

impl Default for MqttConfigWindow {
    fn default() -> Self {
        Self {
            broker_ip: "127.0.0.1".into(),
            broker_port: "1883".into(),
            topics: Default::default(),
            new_topic: Default::default(),
            open: true,

            previous_broker_input: Default::default(),
            broker_status: None,
            validation_in_progress: false,
            last_input_change: None,

            discovery_active: false,
            discovered_topics: Default::default(),
            discovery_rx: None,
            discovery_stop: Default::default(),

            broker_validation_receiver: None,
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
