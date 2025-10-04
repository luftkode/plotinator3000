use egui::{CentralPanel, Color32, Frame, Pos2, Stroke, ViewportBuilder, ViewportId};
use std::sync::mpsc::{Receiver, Sender, channel};
use walkers::{HttpTiles, Map, MapMemory, Position, Projector};

/// Persistent map state that survives viewport closure
#[derive(Clone)]
pub struct MapData {
    pub paths: Vec<Vec<Position>>,
    pub highlighted: Option<Position>,
    pub center_position: Position,
    pub zoom: f64,
}

impl Default for MapData {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            highlighted: None,
            center_position: Position::new(-0.1278, 51.5074), // London (lon, lat)
            zoom: 10.0,
        }
    }
}

/// Messages sent from main app to map viewport
pub enum MapCommand {
    SetPosition(Position),
    AddPath(Vec<Position>),
    SetHighlight(Option<Position>),
    FitToAllPaths,
}

/// Messages sent from map viewport back to main app (for state sync)
pub enum MapUpdate {
    StateChanged { center: Position, zoom: f64 },
}

/// Lightweight state for communicating with the viewport
#[derive(Clone)]
pub struct MapState {
    pub command_sender: Sender<MapCommand>,
}

impl MapState {
    pub fn new(sender: Sender<MapCommand>) -> Self {
        Self {
            command_sender: sender,
        }
    }

    pub fn set_position(&self, lat: f64, lon: f64) {
        let _ = self
            .command_sender
            .send(MapCommand::SetPosition(Position::new(lon, lat)));
    }

    pub fn add_path(&self, coords: Vec<(f64, f64)>) {
        let positions: Vec<Position> = coords
            .iter()
            .map(|(lat, lon)| Position::new(*lon, *lat))
            .collect();
        let _ = self.command_sender.send(MapCommand::AddPath(positions));
    }

    pub fn highlight(&self, lat: f64, lon: f64) {
        let _ = self
            .command_sender
            .send(MapCommand::SetHighlight(Some(Position::new(lon, lat))));
    }

    pub fn clear_highlight(&self) {
        let _ = self.command_sender.send(MapCommand::SetHighlight(None));
    }
}

/// Viewport-specific state (tiles, memory) - recreated each time viewport opens
pub struct MapViewportState {
    map_memory: MapMemory,
    tiles: HttpTiles,
    command_receiver: Receiver<MapCommand>,
    update_sender: Sender<MapUpdate>,
}

pub struct AppWithMap {
    app: crate::App,
    map_state: Option<MapState>,
    map_viewport_state: Option<MapViewportState>,

    // Persistent map data that survives viewport closure
    map_data: MapData,

    // Channel for receiving updates from viewport
    update_receiver: Option<Receiver<MapUpdate>>,
}

impl AppWithMap {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            app: crate::App::new(cc),
            map_state: None,
            map_viewport_state: None,
            map_data: MapData::default(),
            update_receiver: None,
        }
    }

    pub fn set_map_position(&mut self, lat: f64, lon: f64) {
        if let Some(state) = &self.map_state {
            state.set_position(lat, lon);
        }
    }

    pub fn add_path(&self, coords: Vec<(f64, f64)>) {
        if let Some(state) = &self.map_state {
            state.add_path(coords);
        }
    }

    pub fn highlight_on_map(&mut self, lat: f64, lon: f64) {
        if let Some(state) = &self.map_state {
            state.highlight(lat, lon);
        }
    }

    fn open_map_viewport(&mut self, ctx: &egui::Context) {
        if self.map_viewport_state.is_some() {
            return; // Already open
        }

        egui_extras::install_image_loaders(ctx);

        let tiles = HttpTiles::new(walkers::sources::OpenStreetMap, ctx.clone());

        let mut map_memory = MapMemory::default();
        map_memory.center_at(self.map_data.center_position);
        let _ = map_memory.set_zoom(self.map_data.zoom);

        // Create channels for bidirectional communication
        let (cmd_sender, cmd_receiver) = channel();
        let (update_sender, update_receiver) = channel();

        self.map_state = Some(MapState::new(cmd_sender));
        self.update_receiver = Some(update_receiver);

        self.map_viewport_state = Some(MapViewportState {
            map_memory,
            tiles,
            command_receiver: cmd_receiver,
            update_sender,
        });
    }

    fn close_map_viewport(&mut self) {
        self.map_viewport_state = None;
        self.map_state = None;
        self.update_receiver = None;
    }

    fn process_map_commands(&mut self, viewport_state: &mut MapViewportState) -> bool {
        let mut needs_repaint = false;

        while let Ok(cmd) = viewport_state.command_receiver.try_recv() {
            match cmd {
                MapCommand::SetPosition(pos) => {
                    self.map_data.center_position = pos;
                    viewport_state.map_memory.center_at(pos);
                    needs_repaint = true;
                }
                MapCommand::AddPath(path) => {
                    if !path.is_empty() {
                        self.map_data.paths.push(path);
                        needs_repaint = true;
                    }
                }
                MapCommand::SetHighlight(highlight) => {
                    self.map_data.highlighted = highlight;
                    needs_repaint = true;
                }
                MapCommand::FitToAllPaths => {
                    fit_map_to_paths(&mut viewport_state.map_memory, &self.map_data.paths);
                    // Update stored position and zoom
                    self.map_data.center_position = viewport_state.map_memory.center();
                    self.map_data.zoom = viewport_state.map_memory.zoom();
                    needs_repaint = true;
                }
            }
        }

        needs_repaint
    }

    fn process_map_updates(&mut self) {
        if let Some(receiver) = &self.update_receiver {
            while let Ok(update) = receiver.try_recv() {
                match update {
                    MapUpdate::StateChanged { center, zoom } => {
                        self.map_data.center_position = center;
                        self.map_data.zoom = zoom;
                    }
                }
            }
        }
    }
}

impl eframe::App for AppWithMap {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Process any coordinate data from the main app
        if let Some(coords) = self.app.coordinates_data.take() {
            for c in coords {
                log::info!("Adding paths");
                self.add_path(c);
            }
        }

        // Process updates from the map viewport
        self.process_map_updates();

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Windows", |ui| {
                    if ui.button("Show Map").clicked() {
                        self.open_map_viewport(ctx);
                        ui.close();
                    }
                });

                ui.separator();
                if ui.button("Go to New York").clicked() {
                    self.set_map_position(40.7128, -74.0060);
                }

                ui.separator();
                if ui.button("Add Path 1 (London-Paris)").clicked() {
                    let path = vec![
                        (51.5074, -0.1278), // London
                        (51.4, -0.5),
                        (51.0, 0.5),
                        (50.5, 1.5),
                        (49.5, 2.0),
                        (48.8566, 2.3522), // Paris
                    ];
                    self.add_path(path);
                }

                ui.separator();
                if ui.button("Fit Map to Paths").clicked() {
                    if let Some(state) = &self.map_state {
                        let _ = state.command_sender.send(MapCommand::FitToAllPaths);
                    }
                }
            });
        });

        // Show the map viewport if it exists
        if let Some(viewport_state) = &mut self.map_viewport_state {
            // Process commands and check if we need to repaint
            let needs_repaint = self.process_map_commands(viewport_state);
            if needs_repaint {
                ctx.request_repaint();
            }

            let mut open = true;

            ctx.show_viewport_immediate(
                ViewportId::from_hash_of("map_viewport"),
                ViewportBuilder::default()
                    .with_title("Map View")
                    .with_inner_size([800.0, 600.0]),
                |ctx, _class| {
                    // Check if user requested to close the window
                    if ctx.input(|i| i.viewport().close_requested()) {
                        open = false;
                    }

                    CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                        let map = Map::new(
                            None,
                            &mut viewport_state.map_memory,
                            self.map_data.center_position,
                        )
                        .with_layer(&mut viewport_state.tiles, 1.0);

                        map.show(ui, |ui, projector, _map_rect| {
                            // Draw all paths
                            for (idx, path) in self.map_data.paths.iter().enumerate() {
                                if path.len() >= 2 {
                                    let color = get_path_color(idx);
                                    draw_path(ui, &projector, path, color);
                                }
                            }

                            // Draw highlighted point if set
                            if let Some(highlight) = self.map_data.highlighted {
                                let pos2 = projector.project(highlight).to_pos2();
                                let painter = ui.painter();

                                painter.circle_filled(pos2, 6.0, Color32::WHITE);
                                painter.circle_stroke(pos2, 8.0, Stroke::new(3.0, Color32::YELLOW));
                            }
                        });

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "Lat: {:.6}, Lon: {:.6}",
                                self.map_data.center_position.y(),
                                self.map_data.center_position.x()
                            ));
                            ui.separator();
                            ui.label(format!("Paths: {}", self.map_data.paths.len()));
                            ui.separator();
                            ui.label(format!("Zoom: {:.2}", viewport_state.map_memory.zoom()));
                        });
                    });

                    // Sync state back to main app when user pans/zooms
                    let current_center = viewport_state.map_memory.center();
                    let current_zoom = viewport_state.map_memory.zoom();
                    if current_center != self.map_data.center_position
                        || (current_zoom - self.map_data.zoom).abs() > 0.01
                    {
                        let _ = viewport_state.update_sender.send(MapUpdate::StateChanged {
                            center: current_center,
                            zoom: current_zoom,
                        });
                    }
                },
            );

            // Close the viewport if user requested
            if !open {
                self.close_map_viewport();
            }
        }

        self.app.update(ctx, frame);
    }
}

fn fit_map_to_paths(map_memory: &mut MapMemory, paths: &[Vec<Position>]) {
    if paths.is_empty() {
        return;
    }

    let all_positions: Vec<Position> = paths.iter().flat_map(|path| path.iter()).copied().collect();

    if all_positions.is_empty() {
        return;
    }

    // Find bounding box
    let mut min_lat = all_positions[0].y();
    let mut max_lat = all_positions[0].y();
    let mut min_lon = all_positions[0].x();
    let mut max_lon = all_positions[0].x();

    for pos in &all_positions {
        min_lat = min_lat.min(pos.y());
        max_lat = max_lat.max(pos.y());
        min_lon = min_lon.min(pos.x());
        max_lon = max_lon.max(pos.x());
    }

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
}

fn get_path_color(index: usize) -> Color32 {
    let colors = [
        Color32::from_rgb(0, 100, 255),  // Blue
        Color32::from_rgb(50, 200, 50),  // Green
        Color32::from_rgb(255, 150, 0),  // Orange
        Color32::from_rgb(150, 50, 200), // Purple
        Color32::from_rgb(0, 200, 200),  // Cyan
        Color32::from_rgb(255, 200, 0),  // Yellow
    ];
    colors[index % colors.len()]
}

fn draw_path(ui: &mut egui::Ui, projector: &Projector, path: &[Position], color: Color32) {
    if path.len() < 2 {
        return;
    }

    let painter = ui.painter();

    let screen_points: Vec<Pos2> = path
        .iter()
        .map(|pos| projector.project(*pos).to_pos2())
        .collect();

    let stroke = Stroke::new(3.0, color);

    for i in 0..screen_points.len() - 1 {
        painter.line_segment([screen_points[i], screen_points[i + 1]], stroke);
    }

    for point in &screen_points {
        painter.circle_filled(*point, 4.0, color);
    }
}
