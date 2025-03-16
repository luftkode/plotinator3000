use egui_plot::PlotPoint;
use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

use crate::MqttPoint;

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

pub fn mqtt_listener(
    tx: &mpsc::Sender<MqttPoint>,
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
                    let topic = publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    log::info!("Received on topic={topic}, payload={payload}");
                    match payload.parse::<f64>() {
                        Ok(num) => {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("Time went backwards")
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
    if let Err(e) = client.disconnect() {
        log::error!("{e}");
        debug_assert!(false, "{e}");
    }
}
