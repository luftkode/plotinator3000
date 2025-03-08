use chrono::NaiveDateTime;
use egui_plot::PlotPoint;

#[derive(Default)]
pub struct MqttConfigWindow {
    pub broker: String,
    pub topics: Vec<String>,
    pub new_topic: String,
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

use rumqttc::{Client, Event, MqttOptions, Packet, QoS};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

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
