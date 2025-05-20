use egui::Color32;
use egui::RichText;
use egui::ScrollArea;
use egui::Ui;
use plotinator_mqtt::{BrokerStatus, broker_validator::ValidatorStatus};
use plotinator_mqtt::{MqttConfigWindow, MqttDataReceiver};

use crate::util::theme_color;

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
        .scroll([false, true])
        .show(ctx, |ui| {
            ui.columns(2, |columns| {
                show_broker_config_column(&mut columns[0], mqtt_cfg_window);
                show_subscribed_topics_column(
                    &mut columns[1],
                    mqtt_cfg_window,
                    &mut connect_clicked,
                    &mut data_receiver,
                );
            });
        });
    // 4. Cleanup when window closes
    if (!*mqtt_cfg_window_open || connect_clicked) && mqtt_cfg_window.discovery_active() {
        mqtt_cfg_window.stop_topic_discovery();
    }
    data_receiver
}

fn show_broker_config_column(ui: &mut Ui, mqtt_cfg_window: &mut MqttConfigWindow) {
    ui.group(|ui| {
        ui.label("MQTT Broker Address");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(mqtt_cfg_window.broker_host_as_mut())
                .on_hover_text("IP address, hostname, or mDNS (.local)");
            ui.label(":");
            ui.text_edit_singleline(mqtt_cfg_window.broker_port_as_mut())
                .on_hover_text("1883 is the default MQTT broker port");
        });

        match mqtt_cfg_window.validator_status() {
            ValidatorStatus::Inactive => show_broker_status(ui, mqtt_cfg_window.broker_status()),
            ValidatorStatus::Connecting => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Checking for broker...");
                });
            }
            ValidatorStatus::RetrievingVersion => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Retrieving broker version...");
                });
            }
        }

        mqtt_cfg_window.poll_broker_status();

        ui.label("Topics:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(mqtt_cfg_window.text_input_topic_as_mut());
            if ui.button("Add").clicked() {
                mqtt_cfg_window.add_text_input_topic();
            }
        });

        show_discovered_topics_section(ui, mqtt_cfg_window);
    });
}

fn show_subscribed_topics_column(
    ui: &mut Ui,
    mqtt_cfg_window: &mut MqttConfigWindow,
    connect_clicked: &mut bool,
    data_receiver: &mut Option<MqttDataReceiver>,
) {
    ui.group(|ui| {
        let is_connect_valid = mqtt_cfg_window.broker_status().reachable()
            && !mqtt_cfg_window.selected_topics().is_empty();
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    is_connect_valid,
                    egui::Button::new(RichText::new("Connect").strong())
                        .min_size([120.0, 30.0].into()),
                )
                .clicked()
            {
                *connect_clicked = true;
                *data_receiver = Some(mqtt_cfg_window.spawn_mqtt_listener());
            }
        });
        show_subscribed_topics(ui, mqtt_cfg_window);
    });
}

fn show_broker_status(ui: &mut Ui, broker_status: &BrokerStatus) {
    match broker_status {
        BrokerStatus::None => (),
        BrokerStatus::Reachable => {
            draw_reachable_label(ui, None);
        }
        BrokerStatus::Unreachable(err) => {
            ui.colored_label(
                egui::Color32::RED,
                RichText::new(format!(
                    "{icon} {err}",
                    icon = egui_phosphor::regular::WARNING_OCTAGON
                )),
            );
        }
        BrokerStatus::ReachableVersion(v) => {
            draw_reachable_label(ui, Some(v.as_ref()));
        }
    }
}

fn draw_reachable_label(ui: &mut Ui, version: Option<&str>) {
    ui.colored_label(
        egui::Color32::GREEN,
        RichText::new(format!(
            "{icon} {desc}",
            icon = egui_phosphor::regular::CHECK,
            desc = version.unwrap_or("Broker reachable")
        )),
    );
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
        ui.colored_label(
            theme_color(ui, Color32::CYAN, Color32::BLUE),
            "Discovering topics...",
        );
    });

    // Process incoming topics
    if let Err(e) = mqtt_cfg_window.poll_discovered_topics() {
        ui.colored_label(Color32::RED, e);
    }
}

fn show_subscribed_topics(ui: &mut Ui, mqtt_cfg_window: &mut MqttConfigWindow) {
    let subscribed_topics = mqtt_cfg_window.selected_topics().len();
    let label_txt = if subscribed_topics == 0 {
        RichText::new("Select topics to subscribe to before connecting").color(theme_color(
            ui,
            Color32::YELLOW,
            Color32::ORANGE,
        ))
    } else {
        RichText::new(format!("Subscribed Topics ({subscribed_topics}):"))
    };
    ui.label(label_txt);
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

fn show_discovered_topics_list(
    ui: &mut Ui,
    mqtt_cfg_window: &mut MqttConfigWindow,
    topics: &[String],
) {
    ScrollArea::vertical().max_height(800.0).show(ui, |ui| {
        for topic in topics {
            if !mqtt_cfg_window.selected_topics_contains(topic) {
                ui.horizontal(|ui| {
                    if ui.selectable_label(false, topic).clicked() {
                        mqtt_cfg_window.add_selected_topic(topic.to_string());
                    }
                });
            }
        }
    });
}

fn show_discovered_topics_section(ui: &mut Ui, mqtt_cfg_window: &mut MqttConfigWindow) {
    let discover_enabled =
        mqtt_cfg_window.broker_status().reachable() && !mqtt_cfg_window.discovery_active();

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
    let discovered_topics = mqtt_cfg_window.discovered_topics().len();
    if discovered_topics > 0 {
        ui.separator();
        ui.label(format!("Discovered Topics ({discovered_topics})"));

        show_discovered_topics_list(
            ui,
            mqtt_cfg_window,
            &mqtt_cfg_window.discovered_topics_sorted(),
        );
    }

    let discovered_sys_topics = mqtt_cfg_window.discovered_sys_topics().len();
    if discovered_sys_topics > 0 {
        ui.collapsing(
            format!("Broker sys topics ({discovered_sys_topics})"),
            |ui| {
                ui.separator();
                show_discovered_topics_list(
                    ui,
                    mqtt_cfg_window,
                    &mqtt_cfg_window.discovered_sys_topics_sorted(),
                );
            },
        );
    }
}
