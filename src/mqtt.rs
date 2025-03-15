use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use egui::Color32;
use egui::RichText;
use egui::ScrollArea;

pub fn show_mqtt_window(ctx: &egui::Context, app: &mut crate::App) {
    if let Some(config) = &mut app.mqtt_config_window {
        let mut connect_clicked = false;
        egui::Window::new("MQTT Configuration")
            .open(&mut config.open)
            .show(ctx, |ui| {
                ui.group(|ui| {
                    ui.label("MQTT Broker Address");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut config.broker_ip)
                            .on_hover_text("IP address, hostname, or mDNS (.local)");
                        ui.label(":");
                        ui.text_edit_singleline(&mut config.broker_port)
                            .on_hover_text("1883 is the default MQTT broker port");
                    });
                    if let Some(status) = &config.broker_status {
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
                    } else if config.validation_in_progress {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Checking broker...");
                        });
                    }

                    let current_broker_input =
                        format!("{}:{}", config.broker_ip, config.broker_port);

                    // Detect input changes
                    if current_broker_input != config.previous_broker_input {
                        config.previous_broker_input = current_broker_input.clone();
                        config.last_input_change = Some(Instant::now());
                        config.broker_status = None;
                    }

                    // Debounce and validate after 500ms
                    if let Some(last_change) = config.last_input_change {
                        if last_change.elapsed() >= Duration::from_millis(500)
                            && !config.validation_in_progress
                        {
                            let (tx, rx) = std::sync::mpsc::channel();
                            config.broker_validation_receiver = Some(rx);
                            config.validation_in_progress = true;
                            config.last_input_change = None;

                            // Spawn validation thread
                            let (host, port) =
                                (config.broker_ip.clone(), config.broker_port.clone());
                            std::thread::spawn(move || {
                                let result = mqtt::util::validate_broker(&host, &port);
                                tx.send(result).ok();
                            });
                        }
                    }

                    // Check for validation results
                    if let Some(receiver) = &mut config.broker_validation_receiver {
                        if let Ok(result) = receiver.try_recv() {
                            config.broker_status = Some(result);
                            config.validation_in_progress = false;
                            config.broker_validation_receiver = None;
                        }
                    }
                    ui.label("Topics:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut config.new_topic);
                        if ui.button("Add").clicked() && !config.new_topic.is_empty() {
                            config.topics.push(config.new_topic.clone());
                            config.new_topic.clear();
                        }
                    });

                    let discover_enabled =
                        matches!(config.broker_status, Some(Ok(()))) && !config.discovery_active;

                    if let Ok(port_u16) = config.broker_port.parse::<u16>() {
                        if !config.discovery_active
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
                            config.discovery_active = true;
                            config.discovered_topics.clear();
                            config
                                .discovery_stop
                                .store(false, std::sync::atomic::Ordering::SeqCst);

                            let host = config.broker_ip.clone();
                            let (tx, rx) = mpsc::channel();

                            config.discovery_rx = Some(rx);
                            app.discovery_handle = Some(mqtt::start_discovery(
                                host,
                                port_u16,
                                Arc::clone(&config.discovery_stop),
                                tx,
                            ));
                        }
                    }

                    if config.discovery_active
                        && ui
                            .button(format!(
                                "{} Stop Discovery",
                                egui_phosphor::regular::CELL_TOWER
                            ))
                            .clicked()
                    {
                        config.discovery_stop.store(true, Ordering::SeqCst);
                        config.discovery_active = false;
                    }
                    // Show discovery status
                    if config.discovery_active {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.colored_label(Color32::BLUE, "Discovering topics...");
                        });
                    }

                    // Process incoming topics
                    if let Some(rx) = &mut config.discovery_rx {
                        while let Ok(topic) = rx.try_recv() {
                            if topic.starts_with("!ERROR: ") {
                                config.discovery_active = false;
                                ui.colored_label(Color32::RED, &topic[8..]);
                            } else {
                                config.discovered_topics.insert(topic);
                            }
                        }
                    }

                    // Display discovered topics
                    if !config.discovered_topics.is_empty() {
                        ui.separator();
                        ui.label(format!(
                            "Discovered Topics ({})",
                            config.discovered_topics.len()
                        ));

                        ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                            let mut topics: Vec<_> = config.discovered_topics.iter().collect();
                            topics.sort();

                            for topic in topics {
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(false, topic).clicked() {
                                        if !config.topics.contains(topic) {
                                            config.topics.push(topic.to_string());
                                        }
                                    }
                                });
                            }
                        });
                    }
                });
                if !config.topics.is_empty() {
                    ui.label("Subscribed Topics:");
                }
                for topic in &mut config.topics {
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
                config.topics.retain(|s| !s.is_empty());

                if ui.button("Connect").clicked() {
                    app.auto_scale = true;
                    log::info!("Auto scaling enabled");
                    connect_clicked = true;
                    app.mqtt_stop_flag
                        .store(false, std::sync::atomic::Ordering::SeqCst);

                    let broker = config.broker_ip.clone();
                    let topics = config.topics.clone();
                    let (tx, rx) = std::sync::mpsc::channel();
                    app.mqtt_channel = Some(rx);
                    let thread_stop_flag = Arc::clone(&app.mqtt_stop_flag);
                    std::thread::spawn(move || {
                        mqtt::mqtt_receiver(tx, broker, topics, thread_stop_flag);
                    });
                }
            });
        // 4. Cleanup when window closes
        if (!config.open || connect_clicked) && config.discovery_active {
            config.discovery_stop.store(true, Ordering::SeqCst);
            config.discovery_active = false;
        }
    }
}
