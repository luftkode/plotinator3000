use egui::Color32;
use egui::RichText;
use egui::ScrollArea;
use egui::Ui;
use mqtt::{MqttConfigWindow, MqttDataReceiver};

fn show_broker_status(ui: &mut Ui, broker_status: Option<&Result<(), String>>) {
    if let Some(status) = broker_status {
        match status {
            Ok(()) => {
                ui.colored_label(
                    egui::Color32::GREEN,
                    RichText::new(format!(
                        "{} Broker reachable",
                        egui_phosphor::regular::CHECK
                    )),
                );
            }
            Err(err) => {
                ui.colored_label(
                    egui::Color32::RED,
                    RichText::new(format!("{} {err}", egui_phosphor::regular::WARNING_OCTAGON)),
                );
            }
        }
    }
}

fn show_active_discovery_status(ui: &mut Ui, mqtt_cfg_window: &mut MqttConfigWindow) {
    if ui
        .button(format!(
            "{} Stop Discovery",
            egui_phosphor::regular::CELL_TOWER
        ))
        .clicked()
    {
        mqtt_cfg_window.stop_topic_discovery();
    }
    // Show discovery status
    ui.horizontal(|ui| {
        ui.spinner();
        ui.colored_label(Color32::BLUE, "Discovering topics...");
    });

    // Process incoming topics
    if let Err(e) = mqtt_cfg_window.poll_discovered_topics() {
        ui.colored_label(Color32::RED, e);
    }
}

/// Shows the MQTT configuration window and returns a receiver channel if connect was clicked
pub fn show_mqtt_window(
    ctx: &egui::Context,
    mqtt_cfg_window_open: &mut bool,
    mqtt_cfg_window: &mut MqttConfigWindow,
) -> Option<MqttDataReceiver> {
    let mut data_receiver: Option<MqttDataReceiver> = None;
    let mut connect_clicked = false;
    egui::Window::new("MQTT Configuration")
        .open(mqtt_cfg_window_open)
        .show(ctx, |ui| {
            ui.group(|ui| {
                ui.label("MQTT Broker Address");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(mqtt_cfg_window.broker_host_as_mut())
                        .on_hover_text("IP address, hostname, or mDNS (.local)");
                    ui.label(":");
                    ui.text_edit_singleline(mqtt_cfg_window.broker_port_as_mut())
                        .on_hover_text("1883 is the default MQTT broker port");
                });

                if mqtt_cfg_window.validation_in_progress() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Checking broker...");
                    });
                } else {
                    show_broker_status(ui, mqtt_cfg_window.broker_status());
                }

                mqtt_cfg_window.poll_broker_status();

                ui.label("Topics:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(mqtt_cfg_window.text_input_topic_as_mut());
                    if ui.button("Add").clicked() {
                        mqtt_cfg_window.add_text_input_topic();
                    }
                });

                let discover_enabled = mqtt_cfg_window.broker_status().is_some_and(|s| s.is_ok())
                    && !mqtt_cfg_window.discovery_active();

                if !mqtt_cfg_window.discovery_active()
                    && ui
                        .add_enabled(
                            discover_enabled,
                            egui::Button::new(format!(
                                "{} Discover Topics",
                                egui_phosphor::regular::CELL_TOWER
                            )),
                        )
                        .on_hover_text("Continuously find topics (subscribes to #)")
                        .clicked()
                {
                    mqtt_cfg_window.start_topic_discovery();
                }

                if mqtt_cfg_window.discovery_active() {
                    show_active_discovery_status(ui, mqtt_cfg_window);
                }

                // Display discovered topics
                if !mqtt_cfg_window.discovered_topics().is_empty() {
                    ui.separator();
                    ui.label(format!(
                        "Discovered Topics ({})",
                        mqtt_cfg_window.discovered_topics().len()
                    ));

                    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        let topics: Vec<_> = mqtt_cfg_window.discovered_topics_sorted();

                        for topic in &topics {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(false, topic).clicked() {
                                    mqtt_cfg_window.add_selected_topic(topic.to_string());
                                }
                            });
                        }
                    });
                }
            });
            show_subscribed_topics(ui, mqtt_cfg_window);

            if ui.button("Connect").clicked() {
                connect_clicked = true;
                data_receiver = Some(mqtt_cfg_window.spawn_mqtt_listener());
            }
        });
    // 4. Cleanup when window closes
    if (!*mqtt_cfg_window_open || connect_clicked) && mqtt_cfg_window.discovery_active() {
        mqtt_cfg_window.stop_topic_discovery();
    }
    data_receiver
}

fn show_subscribed_topics(ui: &mut Ui, mqtt_cfg_window: &mut MqttConfigWindow) {
    if !mqtt_cfg_window.selected_topics().is_empty() {
        ui.label("Subscribed Topics:");
    }
    for topic in mqtt_cfg_window.selected_topics_as_mut() {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(egui_phosphor::regular::TRASH))
                .clicked()
            {
                // Make them an empty string and then cleanup empty strings after the loop
                topic.clear();
            } else {
                ui.label(topic.clone());
            }
        });
    }
    mqtt_cfg_window.remove_empty_selected_topics();
}
