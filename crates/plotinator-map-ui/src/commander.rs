use egui::Color32;
use plotinator_log_if::rawplot::path_data::GeoSpatialDataset;
use plotinator_mqtt::data::listener::MqttGeoData;
use plotinator_mqtt_ui::plot::ColoredGeoLaserAltitude;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::sync::mpsc::{Receiver, Sender, channel};

/// Messages sent from main app to map viewport
#[derive(strum_macros::Display)]
pub enum MapCommand {
    /// Geo spatial data from a loaded dataset
    AddGeoData(Box<GeoSpatialDataset>),
    /// Geo spatial data received over MQTT continuously
    MQTTGeoData(Box<SmallVec<[MqttGeoData; 10]>>),
    /// Laser altitudes received over MQTT, which will be attempted to be associated with existing [`MqttGeoData`]
    MQTTGeoAltitudes(Box<SmallVec<[ColoredGeoLaserAltitude; 10]>>),
    /// Pointer position on the time axis
    PointerPos(f64),
    FitToAllPaths,
    /// Remove all [`GeoSpatialData`]
    Reset,
}

/// Messages sent to the main plot app
#[derive(strum_macros::Display, Clone, Copy)]
pub enum PlotMessage {
    /// The timestamp of a [`GeoPoint`] near the pointer on the map
    ///
    /// if None, it clears the timestamp, to signify that the pointer is no longer hovering near a point on the map
    PointerTimestamp(Option<(f64, Color32)>),
}

#[derive(Serialize, Deserialize)]
pub struct MapUiCommander {
    /// Whether or not the Map is open for commands
    ///
    /// should be in sync with the [`MapViewPort`]
    open: bool,
    // Have we received geospatial data at any time? We use this to open the map the first time
    // geo spatial data is received, but we don't wanna keep opening it up if the user chose to close it
    pub any_primary_data_received: bool,
    /// Set by a click on the map button, toggles visibility of the map viewport
    pub map_button_clicked: bool,
    #[serde(skip)]
    tx: Option<Sender<MapCommand>>,
    #[serde(skip)]
    rx: Option<Receiver<PlotMessage>>,
    // Used when the app was created with restored state, until the first time the
    // map is opened, where all the pending commands are then sent
    #[serde(skip)]
    tmp_queue: Option<Vec<MapCommand>>,
    #[serde(skip)]
    map_pointer_timestamp_and_color: Option<(f64, Color32)>,
}

impl Default for MapUiCommander {
    fn default() -> Self {
        Self {
            open: false,
            any_primary_data_received: false,
            map_button_clicked: false,
            tx: None,
            rx: None,
            tmp_queue: Some(vec![]),
            map_pointer_timestamp_and_color: None,
        }
    }
}

pub struct MapUiChannels {
    pub map_cmd_tx: Sender<MapCommand>,
    pub map_cmd_rx: Receiver<MapCommand>,
    pub plot_msg_tx: Sender<PlotMessage>,
    pub plot_msg_rx: Receiver<PlotMessage>,
}

impl Default for MapUiChannels {
    fn default() -> Self {
        let (map_cmd_tx, map_cmd_rx) = channel();
        let (plot_msg_tx, plot_msg_rx) = channel();

        Self {
            map_cmd_tx,
            map_cmd_rx,
            plot_msg_tx,
            plot_msg_rx,
        }
    }
}

impl MapUiCommander {
    /// Retrieve channels between [`MapUiCommander`] and the [`MapViewPort`]
    pub fn channels() -> MapUiChannels {
        MapUiChannels::default()
    }

    pub fn init(&mut self, tx: Sender<MapCommand>, rx: Receiver<PlotMessage>) {
        log::debug!("Initializing MapUiCommander");
        debug_assert!(self.tmp_queue.is_some());
        debug_assert!(self.tx.is_none());
        self.rx = Some(rx);
        self.tx = Some(tx);
        if let Some(queue) = self.tmp_queue.take() {
            for pending_cmd in queue.into_iter().rev() {
                self.send_cmd(pending_cmd);
            }
        }
    }

    pub fn add_geo_data(&mut self, geo_data: GeoSpatialDataset) {
        log::debug!("Sending geo data to map: {}", geo_data.name());
        if geo_data.is_primary() && !geo_data.is_empty() {
            self.any_primary_data_received = true;
        }
        self.send_cmd(MapCommand::AddGeoData(Box::new(geo_data)));
    }

    pub fn add_mqtt_geo_points(&mut self, mqtt_points: SmallVec<[MqttGeoData; 10]>) {
        if mqtt_points.iter().any(|data| data.has_coordinates()) {
            self.any_primary_data_received = true;
        }
        self.send_cmd(MapCommand::MQTTGeoData(Box::new(mqtt_points)));
    }

    pub fn add_mqtt_geo_altitudes(
        &mut self,
        mqtt_geo_altitudes: SmallVec<[ColoredGeoLaserAltitude; 10]>,
    ) {
        self.send_cmd(MapCommand::MQTTGeoAltitudes(Box::new(mqtt_geo_altitudes)));
    }

    /// Send the current pointer position on the time axis to the [`MapViewPort`]
    ///
    /// used for highlighting a path point on the map if the time is close enough
    pub fn pointer_time_pos(&mut self, pos: f64) {
        if self.open {
            self.send_cmd(MapCommand::PointerPos(pos));
        }
    }

    /// Fit the Map to include all the loaded paths
    pub fn fit_to_all_paths(&mut self) {
        self.send_cmd(MapCommand::FitToAllPaths);
    }

    /// Reset/remove all the [`GeoSpatialData`] from the map
    pub fn reset_map_data(&mut self) {
        self.send_cmd(MapCommand::Reset);
    }

    fn send_cmd(&mut self, cmd: MapCommand) {
        debug_assert!(
            (self.tx.is_some() && self.tmp_queue.is_none())
                || (self.tx.is_none() && self.tmp_queue.is_some())
        );
        if let Some(tx) = self.tx.as_ref() {
            log::log!(log::Level::Trace, "Sending map command: {cmd}");
            if let Err(e) = tx.send(cmd) {
                log::error!("Failed sending Map command, map is closed: {e}");
                debug_assert!(false);
            }
        } else if let Some(queue) = &mut self.tmp_queue {
            queue.push(cmd);
        }
    }

    /// Close the command channel, should be in sync with whether or not the [`MapViewPort`] is open
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Open the command channel, should be in sync with whether or not the [`MapViewPort`] is open
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

    pub fn poll_msg(&mut self) {
        if let Some(rx) = &mut self.rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    PlotMessage::PointerTimestamp(ts_and_color) => {
                        self.map_pointer_timestamp_and_color = ts_and_color;
                    }
                }
            }
        }
    }

    /// The timestamp and color of the point on the map the pointer is hovering on, if any.
    pub fn map_pointer_timestamp(&self) -> Option<(f64, Color32)> {
        self.map_pointer_timestamp_and_color
    }
}
