use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use egui::Color32;
use egui::RichText;
use egui::ScrollArea;
use mqtt::MqttConfigWindow;
use mqtt::MqttPoint;

pub fn show_mqtt_window(
    ctx: &egui::Context,
    mqtt_cfg_window: &mut MqttConfigWindow,
) -> Option<std::sync::mpsc::Receiver<MqttPoint>> {
    let mut recv_channel: Option<std::sync::mpsc::Receiver<MqttPoint>> = None;

    let mut connect_clicked = false;
    egui::Window::new("MQTT Configuration")
        .open(&mut mqtt_cfg_window.open)
        .show(ctx, |ui| {
            ui.group(|ui| {
                ui.label("MQTT Broker Address");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut mqtt_cfg_window.broker_ip)
                        .on_hover_text("IP address, hostname, or mDNS (.local)");
                    ui.label(":");
                    ui.text_edit_singleline(&mut mqtt_cfg_window.broker_port)
                        .on_hover_text("1883 is the default MQTT broker port");
                });
                if let Some(status) = &mqtt_cfg_window.broker_status {
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
                                RichText::new(format!(
                                    "{} {err}",
                                    egui_phosphor::regular::WARNING_OCTAGON
                                )),
                            );
                        }
                    }
                } else if mqtt_cfg_window.validation_in_progress {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Checking broker...");
                    });
                }

                let current_broker_input = format!(
                    "{}:{}",
                    mqtt_cfg_window.broker_ip, mqtt_cfg_window.broker_port
                );

                // Detect input changes
                if current_broker_input != mqtt_cfg_window.previous_broker_input {
                    mqtt_cfg_window.previous_broker_input = current_broker_input.clone();
                    mqtt_cfg_window.last_input_change = Some(Instant::now());
                    mqtt_cfg_window.broker_status = None;
                }

                // Debounce and validate after 500ms
                if let Some(last_change) = mqtt_cfg_window.last_input_change {
                    if last_change.elapsed() >= Duration::from_millis(500)
                        && !mqtt_cfg_window.validation_in_progress
                    {
                        let (tx, rx) = std::sync::mpsc::channel();
                        mqtt_cfg_window.broker_validation_receiver = Some(rx);
                        mqtt_cfg_window.validation_in_progress = true;
                        mqtt_cfg_window.last_input_change = None;

                        // Spawn validation thread
                        let (host, port) = (
                            mqtt_cfg_window.broker_ip.clone(),
                            mqtt_cfg_window.broker_port.clone(),
                        );
                        std::thread::spawn(move || {
                            let result = mqtt::util::validate_broker(&host, &port);
                            tx.send(result).ok();
                        });
                    }
                }

                // Check for validation results
                if let Some(receiver) = &mut mqtt_cfg_window.broker_validation_receiver {
                    if let Ok(result) = receiver.try_recv() {
                        mqtt_cfg_window.broker_status = Some(result);
                        mqtt_cfg_window.validation_in_progress = false;
                        mqtt_cfg_window.broker_validation_receiver = None;
                    }
                }
                ui.label("Topics:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut mqtt_cfg_window.new_topic);
                    if ui.button("Add").clicked() && !mqtt_cfg_window.new_topic.is_empty() {
                        mqtt_cfg_window
                            .topics
                            .push(mqtt_cfg_window.new_topic.clone());
                        mqtt_cfg_window.new_topic.clear();
                    }
                });

                let discover_enabled = matches!(mqtt_cfg_window.broker_status, Some(Ok(())))
                    && !mqtt_cfg_window.discovery_active;

                if let Ok(port_u16) = mqtt_cfg_window.broker_port.parse::<u16>() {
                    if !mqtt_cfg_window.discovery_active
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
                        mqtt_cfg_window.discovery_active = true;
                        mqtt_cfg_window.discovered_topics.clear();
                        mqtt_cfg_window
                            .discovery_stop
                            .store(false, std::sync::atomic::Ordering::SeqCst);

                        let host = mqtt_cfg_window.broker_ip.clone();
                        let (tx, rx) = mpsc::channel();

                        mqtt_cfg_window.discovery_rx = Some(rx);
                        mqtt_cfg_window.discovery_handle = Some(mqtt::start_discovery(
                            host,
                            port_u16,
                            Arc::clone(&mqtt_cfg_window.discovery_stop),
                            tx,
                        ));
                    }
                }

                if mqtt_cfg_window.discovery_active
                    && ui
                        .button(format!(
                            "{} Stop Discovery",
                            egui_phosphor::regular::CELL_TOWER
                        ))
                        .clicked()
                {
                    mqtt_cfg_window.discovery_stop.store(true, Ordering::SeqCst);
                    mqtt_cfg_window.discovery_active = false;
                }
                // Show discovery status
                if mqtt_cfg_window.discovery_active {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.colored_label(Color32::BLUE, "Discovering topics...");
                    });
                }

                // Process incoming topics
                if let Some(rx) = &mut mqtt_cfg_window.discovery_rx {
                    while let Ok(topic) = rx.try_recv() {
                        if topic.starts_with("!ERROR: ") {
                            mqtt_cfg_window.discovery_active = false;
                            ui.colored_label(Color32::RED, &topic[8..]);
                        } else {
                            mqtt_cfg_window.discovered_topics.insert(topic);
                        }
                    }
                }

                // Display discovered topics
                if !mqtt_cfg_window.discovered_topics.is_empty() {
                    ui.separator();
                    ui.label(format!(
                        "Discovered Topics ({})",
                        mqtt_cfg_window.discovered_topics.len()
                    ));

                    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        let mut topics: Vec<_> = mqtt_cfg_window.discovered_topics.iter().collect();
                        topics.sort();

                        for topic in topics {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(false, topic).clicked() {
                                    if !mqtt_cfg_window.topics.contains(topic) {
                                        mqtt_cfg_window.topics.push(topic.to_string());
                                    }
                                }
                            });
                        }
                    });
                }
            });
            if !mqtt_cfg_window.topics.is_empty() {
                ui.label("Subscribed Topics:");
            }
            for topic in &mut mqtt_cfg_window.topics {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(egui_phosphor::regular::TRASH))
                        .clicked()
                    {
                        topic.clear();
                    } else {
                        ui.label(topic.clone());
                    }
                });
            }
            mqtt_cfg_window.topics.retain(|s| !s.is_empty());

            if ui.button("Connect").clicked() {
                connect_clicked = true;
                mqtt_cfg_window
                    .mqtt_stop_flag
                    .store(false, std::sync::atomic::Ordering::SeqCst);

                let broker = mqtt_cfg_window.broker_ip.clone();
                let topics = mqtt_cfg_window.topics.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                recv_channel = Some(rx);
                let thread_stop_flag = Arc::clone(&mqtt_cfg_window.mqtt_stop_flag);
                std::thread::spawn(move || {
                    mqtt::mqtt_receiver(tx, broker, topics, thread_stop_flag);
                });
            }
        });
    // 4. Cleanup when window closes
    if (!mqtt_cfg_window.open || connect_clicked) && mqtt_cfg_window.discovery_active {
        mqtt_cfg_window.discovery_stop.store(true, Ordering::SeqCst);
        mqtt_cfg_window.discovery_active = false;
    }
    recv_channel
}
