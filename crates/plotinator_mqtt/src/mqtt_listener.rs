use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

use crate::data::listener::MqttData;

pub fn mqtt_listener(
    tx: &mpsc::Sender<MqttData>,
    broker_host: String,
    broker_port: u16,
    topics: Vec<String>,
    stop_flag: &Arc<AtomicBool>,
) {
    let (client, mut connection) = setup_client(broker_host, broker_port);

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
                    handle_event_packet(tx, publish);
                }
            }

            Err(e) => log::error!("{e}"),
        }
    }
    if let Err(e) = client.disconnect() {
        log::error!("{e}");
        debug_assert!(false, "{e}");
    }
}

fn setup_client(broker_host: String, broker_port: u16) -> (rumqttc::Client, rumqttc::Connection) {
    let timestamp_id = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    let mut mqttoptions = MqttOptions::new(
        format!("plotinator3000-{timestamp_id}"),
        broker_host,
        broker_port,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    Client::new(mqttoptions, 100)
}

fn handle_event_packet(tx: &mpsc::Sender<MqttData>, packet: rumqttc::Publish) {
    let topic = packet.topic;
    let payload = String::from_utf8_lossy(&packet.payload);
    log::debug!("Received on topic={topic}, payload={payload}");

    if let Some(mqtt_plot_point) = crate::parse_packet::parse_packet(&topic, &payload) {
        if let Err(e) = tx.send(mqtt_plot_point) {
            log::error!("{e}");
        }
    }
}
