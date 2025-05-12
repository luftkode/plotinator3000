pub use egui::{DroppedFile, Event, Pos2, Rect, accesskit::Role};
pub use egui_kittest::{
    Harness,
    kittest::{Node, Queryable as _},
};
pub use std::path::PathBuf;

pub fn get_harness() -> Harness<'static, plotinator3000::App> {
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
            name: String::new(),
            mime: String::new(),
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
