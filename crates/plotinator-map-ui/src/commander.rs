use plotinator_log_if::prelude::GeoSpatialData;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::MapCommand;

#[derive(Serialize, Deserialize)]
pub struct MapUiCommander {
    /// Whether or not the Map is open for commands
    ///
    /// should be in sync with the [MapViewPort]
    open: bool,
    // Have we received geospatial data at any time?
    pub any_data_received: bool,
    /// Set by a click on the map button, toggles visibility of the map viewport
    pub map_button_clicked: bool,
    #[serde(skip)]
    tx: Option<Sender<MapCommand>>,
    // Used when the app was created with restored state, until the first time the
    // map is opened, where all the pending commands are then sent
    #[serde(skip)]
    tmp_queue: Option<Vec<MapCommand>>,
}

impl Default for MapUiCommander {
    fn default() -> Self {
        Self {
            open: false,
            any_data_received: false,
            map_button_clicked: false,
            tx: None,
            tmp_queue: Some(vec![]),
        }
    }
}

impl MapUiCommander {
    /// Retrieve channels between [MapUiCommander] and the [MapViewPort]
    pub fn channels() -> (Sender<MapCommand>, Receiver<MapCommand>) {
        channel()
    }

    pub fn init(&mut self, tx: Sender<MapCommand>) {
        log::debug!("Initializing MapUiCommander");
        debug_assert!(self.tmp_queue.is_some());
        debug_assert!(self.tx.is_none());
        self.tx = Some(tx);
        if let Some(queue) = self.tmp_queue.take() {
            for pending_cmd in queue.into_iter().rev() {
                self.send_cmd(pending_cmd);
            }
        }
    }

    pub fn add_geo_data(&mut self, geo_data: GeoSpatialData) {
        log::debug!("Sending geo data to map: {}", geo_data.name);
        self.any_data_received = true;
        self.send_cmd(MapCommand::AddGeoData(geo_data));
    }

    /// Send the current cursor position on the time axis to the [MapViewPort]
    ///
    /// used for highlighting a path point on the map if the time is close enough
    pub fn cursor_time_pos(&mut self, pos: f64) {
        if self.open {
            self.send_cmd(MapCommand::CursorPos(pos));
        }
    }

    /// Fit the Map to include all the loaded paths
    pub fn fit_to_all_paths(&mut self) {
        self.send_cmd(MapCommand::FitToAllPaths);
    }

    fn send_cmd(&mut self, cmd: MapCommand) {
        debug_assert!(
            (self.tx.is_some() && self.tmp_queue.is_none())
                || (self.tx.is_none() && self.tmp_queue.is_some())
        );
        if let Some(tx) = self.tx.as_ref() {
            log::debug!(
                "Sending map command: {}",
                match &cmd {
                    MapCommand::AddGeoData(_) => "AddGeoData",
                    MapCommand::CursorPos(_) => "CursorPos",
                    MapCommand::FitToAllPaths => "FitToAllPaths",
                }
            );
            if let Err(e) = tx.send(cmd) {
                log::error!("Failed sending Map command, map is closed: {e}");
                debug_assert!(false);
            }
        } else if let Some(queue) = &mut self.tmp_queue {
            queue.push(cmd);
        }
    }

    /// Close the command channel, should be in sync with whether or not the [MapViewPort] is open
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Open the command channel, should be in sync with whether or not the [MapViewPort] is open
    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Sync the open state with map
    pub fn sync_open(&mut self, is_map_open: bool) {
        self.open = is_map_open;
    }
}
