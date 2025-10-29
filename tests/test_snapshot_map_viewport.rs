#![cfg(not(target_arch = "wasm32"))]
mod util;
use plotinator_hdf5::Tsc;
use plotinator_log_if::prelude::*;
use plotinator_log_if::rawplot::path_data::GeoSpatialDataset;
use plotinator_map_ui::{MapViewPort, commander::MapCommand};
use plotinator_test_util::test_file_defs;
use std::sync::mpsc::Sender;
use util::*;

pub struct MapViewPortHarnessWrapper {
    name: String,
    harness: Harness<'static>,
    cmd_sender: Sender<MapCommand>,
}

impl MapViewPortHarnessWrapper {
    pub fn new(name: impl Into<String>) -> Self {
        let mut map_viewport = MapViewPort::default();
        let (cmd_sender, _msg_receiver) = map_viewport.open();

        let harness = Harness::new(move |ctx| map_viewport.update_direct(ctx));

        Self {
            name: name.into(),
            harness,
            cmd_sender: cmd_sender.expect("was map initialized twice?"),
        }
    }

    /// Run until
    /// - all animations are done
    /// - no more repaints are requested
    ///
    /// Returns the number of frames that were run.
    ///
    /// Note: For map viewports that continuously repaint (e.g., tile downloads),
    /// this will panic. Use `run_steps` instead.
    pub fn run(&mut self) -> u64 {
        self.harness.run()
    }

    /// Run a frame for each queued event (or a single frame if there are no events).
    /// This will call the app closure with each queued event and
    /// update the Harness.
    pub fn step(&mut self) {
        self.harness.step();
    }

    /// Run a number of steps.
    /// Equivalent to calling `step` x times.
    pub fn run_steps(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    /// Save a named snapshot
    ///
    /// Note: Unlike `PlotAppHarnessWrapper`, this does NOT call `fit_contents()`
    /// because context closures cannot do a sizing pass.
    pub fn save_snapshot(&mut self) {
        self.save_snapshot_with_threshold(CiThreshold::default());
    }

    /// Save a named snapshot
    ///
    /// [`CiThreshold`] specifies how much difference we allow in CI before a snapshot diff is an error.
    ///
    /// In CI the snapshot rendering is done on a Mac OS runner, as they are the only ones with
    /// access to a GPU. Typically a threshold of 1-2 is enough to not get false positives,
    /// but for a snapshot that includes a plot with lots of narrow lines (like plotting Mbed PID log)
    /// the threshold will need to be higher.
    ///
    /// Note: Unlike `PlotAppHarnessWrapper`, this does NOT call `fit_contents()`
    /// because context closures cannot do a sizing pass.
    pub fn save_snapshot_with_threshold(&mut self, CiThreshold(threshold): CiThreshold) {
        let is_macos = cfg!(target_os = "macos");
        if std::env::var("CI").is_ok_and(|v| v == "true") {
            // Only macos runners have access to a GPU
            if is_macos {
                eprintln!("Using CI mac OS threshold: {threshold}");
                let opt = egui_kittest::SnapshotOptions::new().threshold(threshold);
                self.harness.snapshot_options(&self.name, &opt);
            }
        } else {
            self.harness.snapshot(&self.name);
        }
    }

    /// Send a command to the `MapViewPort`
    pub fn send_command(&self, cmd: MapCommand) {
        self.cmd_sender
            .send(cmd)
            .expect("failed sending map command");
    }

    /// Add geo-spatial data to the map
    pub fn add_geo_data(&self, geo_data: GeoSpatialDataset) {
        self.send_command(MapCommand::AddGeoData(Box::new(geo_data)));
    }

    pub fn get_screen_rect(&self) -> Rect {
        self.harness.ctx.content_rect()
    }
}

#[test]
fn test_snapshot_render_map_default() {
    let mut harness = MapViewPortHarnessWrapper::new("default_map_window");
    harness.run_steps(4);
    harness.save_snapshot();
}

#[test]
fn test_snapshot_render_map_with_tsc_geo_data() {
    let mut harness = MapViewPortHarnessWrapper::new("map_with_tsc_geo_data");

    let tsc = Tsc::from_path(test_file_defs::tsc::tsc()).unwrap();
    for p in tsc.raw_plots() {
        match p {
            RawPlot::Generic { .. } => (),
            RawPlot::GeoSpatialDataset(geo_spatial_dataset) => {
                harness.add_geo_data(geo_spatial_dataset.clone());
            }
        };
    }

    harness.run_steps(4);
    harness.save_snapshot_with_threshold(CiThreshold(5.));
}
