use std::sync::mpsc::Receiver;

use crate::{MqttPoint, MqttPoints};

pub struct MqttDataReceiver {
    mqtt_plot_data: Vec<MqttPoints>,
    recv: Receiver<MqttPoint>,
}

impl MqttDataReceiver {
    pub fn new(recv: Receiver<MqttPoint>) -> Self {
        Self {
            mqtt_plot_data: Vec::new(),
            recv,
        }
    }

    pub fn plots(&self) -> &[MqttPoints] {
        &self.mqtt_plot_data
    }

    pub fn poll(&mut self) {
        while let Ok(mqtt_point) = self.recv.try_recv() {
            log::debug!("Got point=[{},{}]", mqtt_point.point.x, mqtt_point.point.y);
            self.insert_data(mqtt_point);
        }
    }

    fn insert_data(&mut self, point: MqttPoint) {
        if let Some(mp) = self
            .mqtt_plot_data
            .iter_mut()
            .find(|mp| mp.topic == point.topic)
        {
            mp.data.push(point.point);
        } else {
            self.mqtt_plot_data.push(MqttPoints {
                topic: point.topic,
                data: vec![point.point],
            });
        }
    }
}
