use std::path::PathBuf;

use egui::{DroppedFile, Event, Pos2, Rect, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{Node, Queryable},
};
use test_util::{mbed_pid_v6_regular, mbed_status_v6_regular};

fn get_harness() -> Harness<'static, plotinator3000::App> {
    Harness::new_eframe(|cc| plotinator3000::App::new(cc))
}

pub struct HarnessWrapper {
    name: String,
    harness: Harness<'static, plotinator3000::App>,
}

impl HarnessWrapper {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            harness: get_harness(),
        }
    }

    pub fn inner(&mut self) -> &mut Harness<'static, plotinator3000::App> {
        &mut self.harness
    }

    pub fn run(&mut self) -> u64 {
        self.harness.run()
    }

    pub fn step(&mut self) {
        self.harness.step();
    }

    /// Save a named snapshot, ensure that contents are fitted before taking the snapshot
    pub fn save_snapshot(&mut self) {
        self.harness.fit_contents();
        self.harness.snapshot(&self.name);
    }

    pub fn drop_file(&mut self, path: PathBuf) {
        let dropped_file = DroppedFile {
            path: Some(path),
            name: "".into(),
            mime: "".into(),
            last_modified: None,
            bytes: None,
        };
        self.harness.input_mut().dropped_files.push(dropped_file);
    }

    pub fn input_event(&mut self, e: Event) {
        self.harness.input_mut().events.push(e);
    }

    pub fn get_screen_rect(&self) -> Rect {
        self.harness.ctx.screen_rect()
    }

    /*
    Convenience getters for Plotinator3000 UI elements
    */

    pub fn get_homepage_node(&self) -> Node<'_> {
        self.harness.get_by_role_and_label(Role::Label, "Homepage")
    }

    pub fn get_mqtt_connect_button(&self) -> Node<'_> {
        self.harness
            .get_by_role_and_label(Role::Button, "MQTT connect")
    }

    pub fn get_mqtt_configuration_window(&self) -> Node<'_> {
        self.harness
            .get_by_role_and_label(Role::Window, "MQTT Configuration")
    }
}

#[test]
fn test_snapshot_open_app() {
    let mut harness = HarnessWrapper::new("default_app_window");
    harness.run();
    let homepage = harness.get_homepage_node();
    let main_window = homepage.parent().unwrap();
    assert_eq!(main_window.role(), Role::Window);
    assert!(main_window.is_focused());

    harness.save_snapshot();
}

#[test]
fn test_snapshot_open_mqtt_config_window_connect_disabled() {
    let mut harness = HarnessWrapper::new("default_mqtt_config_window");
    let mqtt_button = harness.get_mqtt_connect_button();
    assert!(mqtt_button.is_clickable());

    mqtt_button.click();
    harness.run();

    let mqtt_cfg_window = harness.get_mqtt_configuration_window();

    let _broker_addr_txt_input = mqtt_cfg_window
        .get_by(|n| n.role() == Role::TextInput && n.value().is_some_and(|v| v == "127.0.0.1"));

    let connect_button = mqtt_cfg_window.get_by_role_and_label(Role::Button, "Connect");
    assert!(connect_button.is_disabled());

    harness.save_snapshot();
}

#[test]
fn test_snapshot_drop_load_mbed_status_regular_v6() {
    let mut harness = HarnessWrapper::new("dropped_mbed_status_regular_v6");
    harness.drop_file(mbed_status_v6_regular());
    harness.run();
    harness.save_snapshot();
}

#[test]
fn test_snapshot_drop_load_mbed_status_pid_v6_with_cursor_on_plot_window() {
    let mut harness = HarnessWrapper::new("dropped_mbed_pid_regular_v6");
    harness.drop_file(mbed_pid_v6_regular());
    harness.run();

    // Place the cursor in the middle plot area to see that the cursor "alighment-lines" are present
    // across the plot areas
    let center_pos = harness.get_screen_rect().center();
    let left_center_pos = harness.get_screen_rect().left_center();
    let offset_right = left_center_pos.x + center_pos.x / 2.;
    let cursor_pos = Pos2::new(left_center_pos.x + offset_right, left_center_pos.y);
    harness.input_event(egui::Event::PointerMoved(cursor_pos));
    harness.step();

    harness.save_snapshot();
}
