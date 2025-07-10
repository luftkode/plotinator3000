use std::sync::{Arc, atomic::AtomicBool};

use crate::{
    BrokerStatus, MqttDataReceiver,
    broker_validator::{BrokerValidator, ValidatorStatus},
    data_receiver::spawn_mqtt_listener,
    topic_discoverer::TopicDiscoverer,
};
use egui::{Color32, RichText, ScrollArea, Ui};
use plotinator_ui_util::theme_color;

pub(crate) fn show_broker_config_column(
    ui: &mut Ui,
    broker_host: &mut String,
    broker_port: &mut String,
    text_input_topic: &mut String,
    selected_topics: &mut Vec<String>,
    topic_discoverer: &mut TopicDiscoverer,
    broker_validator: &mut BrokerValidator,
) {
    ui.group(|ui: &mut Ui| {
        ui.label("MQTT Broker Address");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(broker_host)
                .on_hover_text("IP address, hostname, or mDNS (.local)");
            ui.label(":");
            ui.text_edit_singleline(broker_port)
                .on_hover_text("1883 is the default MQTT broker port");
        });

        match broker_validator.status() {
            ValidatorStatus::Inactive => show_broker_status(ui, broker_validator.broker_status()),
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

        broker_validator.poll_broker_status(broker_host, broker_port);

        ui.label("Topics:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(text_input_topic);
            if ui.button("Add").clicked()
                && !text_input_topic.is_empty()
                && !selected_topics.contains(text_input_topic)
            {
                selected_topics.push(text_input_topic.clone());
                text_input_topic.clear();
            }
        });

        show_discovered_topics_section(
            ui,
            topic_discoverer,
            broker_validator,
            selected_topics,
            broker_host,
            broker_port,
        );
    });
}

#[allow(
    clippy::too_many_arguments,
    reason = "TODO: at least the broker host/port should be grouped..."
)]
pub(crate) fn show_subscribed_topics_column(
    ui: &mut Ui,
    broker_reachable_and_some_selected_topics: bool,
    connect_clicked: &mut bool,
    data_receiver: &mut Option<MqttDataReceiver>,
    stop_flag: &mut Arc<AtomicBool>,
    selected_topics: &mut Vec<String>,
    broker_host: &str,
    broker_port: &str,
) {
    ui.group(|ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    broker_reachable_and_some_selected_topics,
                    egui::Button::new(RichText::new("Connect").strong())
                        .min_size([120.0, 30.0].into()),
                )
                .clicked()
            {
                *connect_clicked = true;
                let data_receiver_instance = spawn_mqtt_listener(
                    stop_flag,
                    broker_host.to_owned(),
                    broker_port.to_owned(),
                    selected_topics,
                );
                *data_receiver = Some(data_receiver_instance);
            }
        });
        show_subscribed_topics(ui, selected_topics);
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

fn show_active_discovery_status(ui: &mut Ui, topic_discoverer: &mut TopicDiscoverer) {
    if ui
        .button(format!(
            "{} Stop Discovery",
            egui_phosphor::regular::CELL_TOWER
        ))
        .clicked()
    {
        topic_discoverer.stop();
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
    if let Err(e) = topic_discoverer.poll_discovered_topics() {
        ui.colored_label(Color32::RED, e);
    }
}

fn show_subscribed_topics(ui: &mut Ui, selected_topics: &mut Vec<String>) {
    let num_subscribed_topics = selected_topics.len();
    let label_txt = if num_subscribed_topics == 0 {
        RichText::new("Select topics to subscribe to before connecting").color(theme_color(
            ui,
            Color32::YELLOW,
            Color32::ORANGE,
        ))
    } else {
        RichText::new(format!("Subscribed Topics ({num_subscribed_topics}):"))
    };
    ui.label(label_txt);
    for topic in selected_topics.iter_mut() {
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
    selected_topics.retain(|t| !t.is_empty());
}

fn show_discovered_topics_list(ui: &mut Ui, selected_topics: &mut Vec<String>, topics: &[String]) {
    ScrollArea::vertical().max_height(800.0).show(ui, |ui| {
        for topic in topics {
            if !selected_topics.contains(topic) {
                ui.horizontal(|ui| {
                    if ui.selectable_label(false, topic).clicked()
                        && !topic.is_empty()
                        && !selected_topics.contains(topic)
                    {
                        selected_topics.push(topic.clone());
                    }
                });
            }
        }
    });
}

fn show_discovered_topics_section(
    ui: &mut Ui,
    topic_discoverer: &mut TopicDiscoverer,
    broker_validator: &BrokerValidator,
    selected_topics: &mut Vec<String>,
    broker_host: &str,
    broker_port: &str,
) {
    let discover_enabled =
        broker_validator.broker_status().reachable() && !topic_discoverer.active();
    if !topic_discoverer.active()
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
        if let Ok(port) = broker_port.parse::<u16>() {
            topic_discoverer.start(broker_host.to_owned(), port);
        }
    }

    if topic_discoverer.active() {
        show_active_discovery_status(ui, topic_discoverer);
    }

    // Display discovered topics
    let discovered_topics = topic_discoverer.discovered_topics().len();
    if discovered_topics > 0 {
        ui.separator();
        ui.label(format!("Discovered Topics ({discovered_topics})"));

        show_discovered_topics_list(
            ui,
            selected_topics,
            &topic_discoverer.discovered_topics_sorted(),
        );
    }

    let discovered_sys_topics = topic_discoverer.discovered_sys_topics().len();
    if discovered_sys_topics > 0 {
        ui.collapsing(
            format!("Broker sys topics ({discovered_sys_topics})"),
            |ui| {
                ui.separator();
                show_discovered_topics_list(
                    ui,
                    selected_topics,
                    &topic_discoverer.discovered_sys_topics_sorted(),
                );
            },
        );
    }
}
