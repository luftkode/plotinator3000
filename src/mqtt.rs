use std::time::Duration;

use egui::{Color32, RichText};

pub(crate) fn show_mqtt_connect_button(
    app: &mut crate::App,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
) {
    let mqtt_connect_button_txt = if app.mqtt.active_and_connected() {
        RichText::new(format!(
            "{} MQTT connect",
            egui_phosphor::regular::WIFI_HIGH
        ))
        .color(Color32::GREEN)
    } else if app.mqtt.active_but_disconnected() {
        RichText::new(format!(
            "{} MQTT connect",
            egui_phosphor::regular::WIFI_SLASH
        ))
        .color(Color32::RED)
    } else {
        RichText::new("MQTT connect".to_owned())
    };
    if app.mqtt.active_but_disconnected() {
        ui.spinner();
    }
    if ui.button(mqtt_connect_button_txt).clicked() {
        app.mqtt.connect();
    }

    if app.mqtt.listener_active() {
        app.mqtt.poll_data();
        ctx.request_repaint_after(Duration::from_millis(50));
    }
    // Show MQTT configuration window if needed
    app.mqtt.show_connect_window(ui);
}
