use chrono::NaiveDateTime;
use egui_plot::PlotPoint;

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

use rumqttc::{Client, MqttOptions, QoS};
use std::thread;
use std::time::Duration;

pub fn mqtt_receiver(tx: std::sync::mpsc::Sender<MqttPoint>) {
    let mut mqttoptions = MqttOptions::new("plotinator3000", "192.168.0.200", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut connection) = Client::new(mqttoptions, 10);
    client.subscribe("system/pi0/#", QoS::AtMostOnce).unwrap();

    loop {
        // Iterate to poll the eventloop for connection progress
        for notification in connection.iter() {
            match notification {
                Ok(event) => {
                    if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) = event {
                        let topic = publish.topic;
                        let payload = String::from_utf8_lossy(&publish.payload);
                        log::info!("Received on topic={topic}, payload={payload}");
                        if let Ok(num) = payload.parse::<f64>() {
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
                    }
                }

                Err(e) => log::error!("{e}"),
            }
        }
    }
}
