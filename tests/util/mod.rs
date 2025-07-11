#![allow(
    clippy::disallowed_types,
    reason = "This is test utilities so things like PathBuf is fine, we won't deploy this code anywhere"
)]
pub use egui::{DroppedFile, Event, Pos2, Rect, accesskit::Role};
pub use egui_kittest::{Harness, kittest::Queryable as _};
pub use egui_kittest::{
    Node,
    kittest::{NodeT as _, Queryable as _},
};
pub use std::path::PathBuf;

pub fn get_harness() -> Harness<'static, plotinator3000::App> {
    Harness::new_eframe(|cc| plotinator3000::App::new(cc))
}

const DEFAULT_CI_DIFF_THRESHOLD: f32 = 1.0;

/// specifies how much difference we allow in CI before a snapshot diff is an error.
///
/// Default is `1.0`
#[derive(Clone, Copy)]
pub struct CiThreshold(pub f32);

impl Default for CiThreshold {
    fn default() -> Self {
        Self(DEFAULT_CI_DIFF_THRESHOLD)
    }
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
        self.save_snapshot_with_threshold(CiThreshold::default());
    }

    /// Save a named snapshot, ensure that contents are fitted before taking the snapshot
    ///
    /// [`CiThreshold`] specifies how much difference we allow in CI before a snapshot diff is an error.
    ///
    /// In CI the snapshot rendering is done on a Mac OS runner, as they are the only ones with
    /// access to a GPU. Typically a threshold of 1-2 is enough to not get false positives,
    /// but for a snapshot that includes a plot with lots of narrow lines (like plotting Mbed PID log)
    /// the threshold will need to be higher.
    pub fn save_snapshot_with_threshold(&mut self, threshold: CiThreshold) {
        let is_macos = cfg!(target_os = "macos");
        self.harness.fit_contents();

        if std::env::var("CI").is_ok_and(|v| v == "true") {
            // Only macos runners have access to a GPU
            if is_macos {
                eprintln!("Using CI mac OS threshold: {}", threshold.0);
                let opt = egui_kittest::SnapshotOptions::new().threshold(threshold.0);
                self.harness.snapshot_options(&self.name, &opt);
            }
        } else {
            self.harness.snapshot(&self.name);
        }
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
