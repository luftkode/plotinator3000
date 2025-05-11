use plotinator_mqtt::{MqttConfigWindow, MqttDataReceiver, MqttPlotPoints};

pub mod mqtt_window;

#[derive(Default)]
pub struct Mqtt {
    pub mqtt_data_receiver: Option<MqttDataReceiver>,
    mqtt_config_window: Option<MqttConfigWindow>,
    mqtt_cfg_window_open: bool,
    // auto scale plot bounds (MQTT only)
    pub set_auto_bounds: bool,
}

impl Mqtt {
    pub fn reset(&mut self) {
        self.mqtt_data_receiver = None;
        self.mqtt_config_window = None;
    }

    pub fn connect(&mut self) {
        self.mqtt_config_window = Some(plotinator_mqtt::MqttConfigWindow::default());
        self.mqtt_cfg_window_open = true;
    }

    pub fn poll_data(&mut self) {
        self.mqtt_data_receiver
            .as_mut()
            .expect("Attempted to poll when no listener is active")
            .poll();
    }

    pub fn window_open(&self) -> bool {
        self.mqtt_cfg_window_open
    }

    pub fn window_open_mut(&mut self) -> &mut bool {
        &mut self.mqtt_cfg_window_open
    }

    pub fn plots(mqtt_data_receiver: Option<&MqttDataReceiver>) -> &[MqttPlotPoints] {
        mqtt_data_receiver
            .as_ref()
            .map(|mdc| mdc.plots())
            .unwrap_or_default()
    }

    pub fn listener_active(&self) -> bool {
        self.mqtt_data_receiver.is_some()
    }

    /// Returns true if we're listening for MQTT data but have yet to receive enough to display a plot
    ///
    /// One topic needs at least 2 points for us to have anything to plot
    fn waiting_for_initial_data(&self) -> bool {
        if let Some(r) = &self.mqtt_data_receiver {
            for p in r.plots() {
                if p.data.len() > 1 {
                    // A topic has more than 1 plot point so we are no longer waiting
                    return false;
                }
            }
            // We are receiving MQTT data but no topic has 2 plot points or more
            return true;
        }
        // We are not receiving MQTT data, so we are not waiting for it
        false
    }

    pub fn show_waiting_for_initial_data(&self, ui: &mut egui::Ui) {
        if self.waiting_for_initial_data() {
            ui.vertical_centered_justified(|ui| {
                ui.heading("Waiting for data on any of the following topics:");
                debug_assert!(self.mqtt_data_receiver.is_some(), "Expected an active MQTT data receiver when painting 'waiting for initial data' elements");
                for topic in self.mqtt_data_receiver.as_ref().expect("Unsound condition").subscribed_topics() {
                    ui.label(topic);
                }
                ui.spinner();
            });
        }
    }

    pub fn show_connect_window(&mut self, ctx: &egui::Context) {
        if self.mqtt_data_receiver.is_none() {
            if let Some(config) = &mut self.mqtt_config_window {
                if let Some(data_receiver) = crate::mqtt::mqtt_window::show_mqtt_window(
                    ctx,
                    &mut self.mqtt_cfg_window_open,
                    config,
                ) {
                    self.mqtt_data_receiver = Some(data_receiver);
                    self.set_auto_bounds = true;
                }
            }
        }
    }
}
