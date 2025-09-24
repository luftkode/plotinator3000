use std::time::Duration;

use egui::{Color32, RichText};
use egui_phosphor::regular::{WIFI_HIGH, WIFI_SLASH};
use plotinator_mqtt_ui::connection::MqttConnectionMode;

pub(crate) fn show_mqtt_connect_button(
    app: &mut crate::App,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    for mode in app.mqtt.connection_modes() {
        let label_txt = match mode {
            MqttConnectionMode::ActiveAndConnected { broker_host } => {
                RichText::new(format!("{WIFI_HIGH} {broker_host}")).color(Color32::GREEN)
            }
            MqttConnectionMode::ActiveButDisconnected { broker_host } => {
                ui.spinner();
                RichText::new(format!("{WIFI_SLASH} {broker_host}")).color(Color32::RED)
            }
            MqttConnectionMode::Inactive => continue,
        };
        ui.label(label_txt);
    }

    if ui.button("MQTT connect").clicked() {
        app.mqtt.connect();
    }

    if app.mqtt.listener_active() {
        app.mqtt.poll_data();
        ctx.request_repaint_after(Duration::from_millis(50));
    }
    // Show MQTT configuration window if needed
    app.mqtt.show_connect_window(ui);
}
