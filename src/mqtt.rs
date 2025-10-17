use std::time::Duration;

use egui::{Button, Color32, RichText};
use egui_phosphor::regular::{PAPER_PLANE_RIGHT, WIFI_HIGH, WIFI_SLASH};
use plotinator_mqtt_ui::connection::MqttConnectionMode;

pub(crate) fn show_mqtt_connect_button(
    app: &mut crate::PlotApp,
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
        let is_scrolling = app.mqtt.plot_scroller.active();
        let mut btn_txt = RichText::new(format!("{PAPER_PLANE_RIGHT} Scroll"));
        if is_scrolling {
            btn_txt = btn_txt.color(Color32::GREEN);
        }
        let follow_data_btn = Button::new(btn_txt);
        if ui
            .add_enabled(!is_scrolling, follow_data_btn)
            .on_hover_text("Scroll the plot area to follow incoming data")
            .clicked()
        {
            app.mqtt.plot_scroller.activate();
        }
    }
    // Show MQTT configuration window if needed
    app.mqtt.show_connect_window(ui);
}
