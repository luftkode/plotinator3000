use egui::{CentralPanel, Color32, Frame, Pos2, Stroke, ViewportBuilder, ViewportId};
use plotinator_log_if::prelude::GeoSpatialData;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender, channel};
use walkers::{HttpTiles, Map, MapMemory, Position, Projector};

/// Messages sent from main app to map viewport
pub enum MapCommand {
    AddGeoData(GeoSpatialData),
    /// Cursor position on the time axis
    CursorPos(f64),
    FitToAllPaths,
}

#[derive(Serialize, Deserialize)]
pub struct MapUiCommander {
    /// Whether or not the Map is open for commands
    ///
    /// should be in sync with the [MapViewPort]
    open: bool,
    // Have we received geospatial data at any time?
    pub any_data_received: bool,
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
}

#[derive(Default, Deserialize, Serialize)]
pub struct MapViewPort {
    pub open: bool,
    map_data: MapData,
    // Cached map data (external), instantiated on first open, loaded on demand
    #[serde(skip)]
    map_tile_state: Option<MapTileState>,
    #[serde(skip)]
    cmd_recv: Option<Receiver<MapCommand>>,
}

impl MapViewPort {
    /// Open the [MapViewPort]
    ///
    /// if it's the first time it's opened, it will start loading map tiles and
    /// return a [Sender<MapCommand>] for interacting with the Map from other contexts
    pub fn open(&mut self, ctx: &egui::Context) -> Option<Sender<MapCommand>> {
        if self.map_tile_state.is_none() {
            egui_extras::install_image_loaders(ctx);

            let tiles = HttpTiles::new(walkers::sources::OpenStreetMap, ctx.clone());
            let mut map_memory = MapMemory::default();
            map_memory.center_at(self.map_data.center_position);
            let _ = map_memory.set_zoom(self.map_data.zoom);
            self.map_tile_state = Some(MapTileState { map_memory, tiles });
        }
        let mut maybe_map_send = None;
        if self.cmd_recv.is_none() {
            let (cmd_send, cmd_recv) = MapUiCommander::channels();
            maybe_map_send = Some(cmd_send);
            self.cmd_recv = Some(cmd_recv);
        }
        self.open = true;

        maybe_map_send
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn poll_commands(&mut self) {
        while let Ok(cmd) = self
            .cmd_recv
            .as_ref()
            .expect("unsound condition")
            .try_recv()
        {
            match cmd {
                MapCommand::AddGeoData(geo_data) => {
                    log::debug!("Received geo data {}", geo_data.name);
                    self.map_data.geo_data.push(geo_data);
                    self.fit_map_to_paths();
                }
                MapCommand::CursorPos(time_pos) => todo!(),
                MapCommand::FitToAllPaths => {
                    self.fit_map_to_paths();
                }
            }
        }
    }

    fn fit_map_to_paths(&mut self) {
        if let Some((center, zoom)) = fit_map_to_paths(
            &mut self.map_tile_state.as_mut().unwrap().map_memory,
            &self.map_data.geo_data,
        ) {
            // Update stored position and zoom
            self.map_data.center_position = center;
            self.map_data.zoom = zoom;
        }
    }

    /// Shows the map viewport and handles its UI.
    /// This is the primary drawing method to be called from your main app's update loop.
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let mut is_still_open = true;

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("map_viewport"),
            ViewportBuilder::default()
                .with_title("Map")
                .with_inner_size([800.0, 600.0]),
            |ctx, _class| {
                // The closure contains all the UI logic for the viewport.
                if ctx.input(|i| i.viewport().close_requested()) {
                    is_still_open = false;
                }

                self.poll_commands();

                let MapTileState { map_memory, tiles } = self.map_tile_state.as_mut().unwrap();

                CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                    let map = Map::new(Some(tiles), map_memory, self.map_data.center_position)
                        .double_click_to_zoom(true);

                    map.show(ui, |ui, projector, _map_rect| {
                        for geo_data in &self.map_data.geo_data {
                            draw_path(ui, &projector, geo_data);
                        }
                    });
                });
            },
        );

        // If the user requested to close the window, update the state.
        if !is_still_open {
            self.close();
        }
    }
}

pub struct MapTileState {
    map_memory: MapMemory,
    tiles: HttpTiles,
}

/// Persistent map state that survives viewport closure
#[derive(Clone, Deserialize, Serialize)]
pub struct MapData {
    pub geo_data: Vec<GeoSpatialData>,
    pub highlighted: Option<Position>,
    pub center_position: Position,
    pub zoom: f64,
}

impl Default for MapData {
    fn default() -> Self {
        Self {
            geo_data: Vec::new(),
            highlighted: None,
            center_position: Position::new(-0.1278, 51.5074), // London (lon, lat)
            zoom: 10.0,
        }
    }
}

fn fit_map_to_paths(
    map_memory: &mut MapMemory,
    geo_data: &[GeoSpatialData],
) -> Option<(Position, f64)> {
    if geo_data.is_empty() {
        return None;
    }

    // Find bounding box
    let mut min_lat: Option<f64> = None;
    let mut max_lat: Option<f64> = None;
    let mut min_lon: Option<f64> = None;
    let mut max_lon: Option<f64> = None;

    for gd in geo_data.iter() {
        let (tmp_min_lat, tmp_max_lat) = gd.lat_bounds();
        let (tmp_min_lon, tmp_max_lon) = gd.lon_bounds();
        if let Some(min_lat) = min_lat.as_mut() {
            *min_lat = min_lat.min(tmp_min_lat);
        } else {
            min_lat = Some(tmp_min_lat);
        }
        if let Some(max_lat) = max_lat.as_mut() {
            *max_lat = max_lat.min(tmp_max_lat);
        } else {
            max_lat = Some(tmp_max_lat);
        }
        if let Some(min_lon) = min_lon.as_mut() {
            *min_lon = min_lon.min(tmp_min_lon);
        } else {
            min_lon = Some(tmp_min_lon);
        }
        if let Some(max_lon) = max_lon.as_mut() {
            *max_lon = max_lon.min(tmp_max_lon);
        } else {
            max_lon = Some(tmp_max_lon);
        }
    }

    let (Some(min_lat), Some(max_lat), Some(min_lon), Some(max_lon)) =
        (min_lat, max_lat, min_lon, max_lon)
    else {
        return None;
    };

    // Calculate center
    let center_lat = (min_lat + max_lat) / 2.0;
    let center_lon = (min_lon + max_lon) / 2.0;
    let center = Position::new(center_lon, center_lat);

    // Calculate appropriate zoom level
    let lat_span = max_lat - min_lat;
    let lon_span = max_lon - min_lon;
    let max_span = lat_span.max(lon_span);

    let zoom = if max_span > 0.0 {
        let padded_span = max_span * 1.5;
        let zoom = (360.0 / padded_span).log2();
        zoom.clamp(2.0, 18.0)
    } else {
        10.0
    };

    map_memory.center_at(center);
    let _ = map_memory.set_zoom(zoom);

    Some((center, zoom))
}

fn draw_path(ui: &mut egui::Ui, projector: &Projector, path: &GeoSpatialData) {
    if path.points.len() < 2 {
        return;
    }

    let painter = ui.painter();
    let color = path.color;

    // Convert geographic coordinates to screen coordinates
    let screen_points: Vec<(Pos2, Option<f64>)> = path
        .points
        .iter()
        .map(|p| (projector.project(p.position).to_pos2(), p.heading))
        .collect();

    // Draw the path as a colored line
    let stroke = Stroke::new(3.0, color);

    for window in screen_points.windows(2) {
        painter.line_segment([window[0].0, window[1].0], stroke);
    }

    // Draw circles and heading arrows at each point
    for (point, heading) in &screen_points {
        painter.circle_stroke(*point, 4.0, Stroke::new(1.0, color));

        // Draw heading arrow if available
        if let Some(heading_deg) = heading {
            draw_heading_arrow(painter, *point, *heading_deg, color);
        }
    }
}

fn draw_heading_arrow(painter: &egui::Painter, center: Pos2, heading_deg: f64, color: Color32) {
    // Convert heading to radians for trigonometric functions.
    // Screen coordinates have Y pointing down, so we subtract from 90 degrees.
    let angle_rad = (90.0 - heading_deg).to_radians() as f32;

    let arrow_length = 12.0_f32;
    let arrow_width = 8.0_f32;

    // Calculate arrow tip position relative to the center
    let tip = center + arrow_length * egui::vec2(angle_rad.cos(), -angle_rad.sin());

    // Calculate the two base corners of the triangle
    let perpendicular_angle = angle_rad + std::f32::consts::PI / 2.0;
    let base_offset = arrow_width / 2.0;
    let base_vec = base_offset * egui::vec2(perpendicular_angle.cos(), -perpendicular_angle.sin());

    let left = center - base_vec;
    let right = center + base_vec;

    // Draw a filled triangle
    let points = vec![tip, left, right];
    painter.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}
