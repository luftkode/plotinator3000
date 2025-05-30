use rumqttc::{Client, Event, MqttOptions, Packet};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

use crate::util::timestamped_client_id;

#[derive(Default)]
pub(crate) struct TopicDiscoverer {
    active: bool,
    discovered_topics: HashSet<String>,
    discovered_sys_topics: HashSet<String>,
    discovery_rx: Option<mpsc::Receiver<DiscoveryMsg>>,
    stop_discovery_flag: Arc<AtomicBool>,
    discovery_handle: Option<std::thread::JoinHandle<()>>,
}

impl TopicDiscoverer {
    pub fn stop(&mut self) {
        self.stop_discovery_flag.store(true, Ordering::SeqCst);
        self.active = false;
    }

    pub fn get_stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_discovery_flag)
    }

    pub fn reset(&mut self) {
        self.discovered_topics.clear();
        self.stop_discovery_flag.store(false, Ordering::SeqCst);
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn poll_discovered_topics(&mut self) -> Result<(), String> {
        if let Some(rx) = &mut self.discovery_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    DiscoveryMsg::Topic(t) => self.discovered_topics.insert(t),
                    DiscoveryMsg::SysTopic(t) => self.discovered_sys_topics.insert(t),
                    DiscoveryMsg::Err(e) => return Err(e),
                };
            }
        }
        Ok(())
    }

    pub fn start(&mut self, broker_host: String, broker_port: u16) {
        self.reset();
        self.active = true;

        let (tx, rx) = mpsc::channel();

        self.discovery_rx = Some(rx);
        self.discovery_handle = Some(start_discovery(
            broker_host,
            broker_port,
            self.get_stop_flag(),
            tx,
        ));
    }

    pub fn discovered_topics(&self) -> &HashSet<String> {
        &self.discovered_topics
    }

    pub fn discovered_topics_sorted(&self) -> Vec<String> {
        let mut topics: Vec<String> = self.discovered_topics.iter().cloned().collect();
        topics.sort();
        topics
    }

    pub fn discovered_sys_topics(&self) -> &HashSet<String> {
        &self.discovered_sys_topics
    }

    pub fn discovered_sys_topics_sorted(&self) -> Vec<String> {
        let mut topics: Vec<String> = self.discovered_sys_topics.iter().cloned().collect();
        topics.sort();
        topics
    }
}

pub(crate) enum DiscoveryMsg {
    Topic(String),
    SysTopic(String),
    Err(String),
}

pub(crate) fn start_discovery(
    host: String,
    port: u16,
    stop_flag: Arc<AtomicBool>,
    tx: mpsc::Sender<DiscoveryMsg>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("mqtt-topic-discoverer".into())
        .spawn(move || {
            let client_id = timestamped_client_id("discover");

            log::info!(
                "Subscribing for discovery with id={client_id}, broker address={host}:{port}"
            );
            let mut mqttoptions = MqttOptions::new(client_id, host, port);
            mqttoptions.set_keep_alive(Duration::from_secs(5));
            let (client, mut connection) = Client::new(mqttoptions, 100);

            if let Err(e) = client.subscribe("#", rumqttc::QoS::AtMostOnce) {
                log::error!("Subscribe err={e}");
                if let Err(e) = tx.send(DiscoveryMsg::Err(e.to_string())) {
                    log::error!("{e}");
                }
                return;
            }

            if let Err(e) = client.subscribe("$SYS/#", rumqttc::QoS::AtMostOnce) {
                log::error!("Subscribe err={e}");
                if let Err(e) = tx.send(DiscoveryMsg::Err(e.to_string())) {
                    log::error!("{e}");
                }
                // Don't error out on this
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
                            let msg = if p.topic.starts_with("$SYS") {
                                DiscoveryMsg::SysTopic(p.topic)
                            } else {
                                DiscoveryMsg::Topic(p.topic)
                            };
                            if let Err(e) = tx.send(msg) {
                                log::error!("{e}");
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Discover connection err={e}");
                        if let Err(e) = tx.send(DiscoveryMsg::Err(e.to_string())) {
                            log::error!("{e}");
                        }
                        break;
                    }
                }
            }
            if let Err(e) = client.disconnect() {
                log::error!("{e}");
                debug_assert!(false, "{e}");
            }
        })
        .expect("Failed to start MQTT topic discoverer thread")
}


