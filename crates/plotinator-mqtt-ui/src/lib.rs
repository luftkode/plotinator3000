use egui::Color32;
use plotinator_mqtt::{
    MqttConfigWindow, MqttDataReceiver, MqttPlotPoints, data::plot::MqttPlotData,
};

#[derive(Default)]
pub struct Mqtt {
    pub mqtt_data_receiver: Option<MqttDataReceiver>,
    mqtt_config_window: Option<MqttConfigWindow>,
    pub mqtt_plot_data: Option<MqttPlotData>,
    // auto scale plot bounds (MQTT only)
    pub set_auto_bounds: bool,
}

impl Mqtt {
    pub fn reset(&mut self) {
        self.mqtt_data_receiver = None;
        self.mqtt_config_window = None;
        self.mqtt_plot_data = None;
    }

    pub fn connect(&mut self) {
        if let Some(win) = &mut self.mqtt_config_window {
            win.is_open = true;
        } else {
            self.mqtt_config_window = Some(plotinator_mqtt::MqttConfigWindow::default());
        }
    }

    pub fn poll_data(&mut self) {
        if self.mqtt_plot_data.is_none() {
            self.mqtt_plot_data = Some(MqttPlotData::default());
        }
        self.mqtt_data_receiver
            .as_mut()
            .expect("Attempted to poll when no listener is active")
            .poll(self.mqtt_plot_data.as_mut().expect("unsound condition"));
    }

    pub fn window_open(&self) -> bool {
        self.mqtt_config_window
            .as_ref()
            .is_some_and(|win| win.is_open)
    }

    pub fn plots(mqtt_plot_data: Option<&MqttPlotData>) -> &[(MqttPlotPoints, Color32)] {
        mqtt_plot_data
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
        if self.mqtt_data_receiver.is_some() {
            if let Some(mqtt_plot_data) = &self.mqtt_plot_data {
                for (p, _) in mqtt_plot_data.plots() {
                    if p.data.len() > 1 {
                        // A topic has more than 1 plot point so we are no longer waiting
                        return false;
                    }
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
            ui.add_space(20.);
            ui.vertical_centered_justified(|ui| {
                ui.heading("Waiting for 2 data points on any of the following topics:");
                debug_assert!(self.mqtt_data_receiver.is_some(), "Expected an active MQTT data receiver when painting 'waiting for initial data' elements");
                for topic in self.mqtt_data_receiver.as_ref().expect("Unsound condition").subscribed_topics() {
                    ui.label(topic);
                }
                ui.spinner();
            });
        }
    }

    pub fn show_connect_window(&mut self, ui: &mut egui::Ui) {
        if let Some(mqtt_config_window) = &mut self.mqtt_config_window {
            if mqtt_config_window.is_open || self.mqtt_data_receiver.is_none() {
                if let Some(data_receiver) = mqtt_config_window.ui(ui) {
                    self.mqtt_data_receiver = Some(data_receiver);
                    self.set_auto_bounds = true;
                }
            }
        }
    }

    pub fn active_and_connected(&self) -> bool {
        self.mqtt_data_receiver
            .as_ref()
            .is_some_and(|receiver| receiver.connected())
    }

    pub fn active_but_disconnected(&self) -> bool {
        self.mqtt_data_receiver
            .as_ref()
            .is_some_and(|receiver| !receiver.connected())
    }
}
