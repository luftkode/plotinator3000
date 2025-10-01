use egui::{CentralPanel, Color32, Frame, Pos2, Stroke, ViewportBuilder, ViewportId};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use walkers::{HttpTiles, Map, MapMemory, Position, Projector};

#[derive(Clone)]
pub struct MapState {
    pub target_position: Arc<Mutex<Position>>,
    pub path_sender: Sender<Vec<Position>>,
}

impl MapState {
    pub fn new(sender: Sender<Vec<Position>>) -> Self {
        Self {
            target_position: Arc::new(Mutex::new(Position::new(51.5074, -0.1278))),
            path_sender: sender,
        }
    }
}

pub struct MapViewportState {
    map_memory: MapMemory,
    tiles: HttpTiles,
    last_set_position: Position,
    paths: Vec<Vec<Position>>,
    path_receiver: Receiver<Vec<Position>>,
    highlighted: Option<Position>,
}

pub struct AppWithMap {
    app: crate::App,
    map_state: MapState,
    map_viewport_state: Option<MapViewportState>,
}

impl AppWithMap {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (sender, _receiver) = channel();

        Self {
            app: crate::App::new(cc),
            map_state: MapState::new(sender),
            map_viewport_state: None,
        }
    }

    pub fn set_map_position(&mut self, lat: f64, lon: f64) {
        let mut position = self.map_state.target_position.lock().unwrap();
        *position = Position::new(lon, lat);
    }

    pub fn add_path(&self, coords: Vec<(f64, f64)>) {
        let sender = &self.map_state.path_sender;
        // Convert (lat, lon) pairs to Position (which takes lon, lat)
        let positions: Vec<Position> = coords
            .iter()
            .map(|(lat, lon)| Position::new(*lon, *lat))
            .collect();

        // Send through channel (ignore errors if receiver is dropped)
        let _ = sender.send(positions);
    }

    pub fn get_path_sender(&self) -> Sender<Vec<Position>> {
        self.map_state.path_sender.clone()
    }
}

impl eframe::App for AppWithMap {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(coords) = self.app.coordinates_data.take() {
            for c in coords {
                log::info!("Adding paths");
                self.add_path(c);
            }
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Windows", |ui| {
                    if ui.button("Show Map").clicked() {
                        if self.map_viewport_state.is_none() {
                            egui_extras::install_image_loaders(ctx);

                            let tiles =
                                HttpTiles::new(walkers::sources::OpenStreetMap, ctx.clone());

                            let mut map_memory = MapMemory::default();
                            let initial_position = *self.map_state.target_position.lock().unwrap();
                            map_memory.center_at(initial_position);

                            // Create a new receiver for the viewport
                            let (sender, receiver) = channel();
                            self.map_state.path_sender = sender;

                            self.map_viewport_state = Some(MapViewportState {
                                map_memory,
                                tiles,
                                last_set_position: initial_position,
                                paths: Vec::new(),
                                path_receiver: receiver,
                                highlighted: None,
                            });
                        }
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
            });
        });

        // Show the map viewport if it exists
        if let Some(viewport_state) = &mut self.map_viewport_state {
            let target = *self.map_state.target_position.lock().unwrap();

            // Receive all pending paths from the channel
            let mut new_paths_added = false;
            while let Ok(new_path) = viewport_state.path_receiver.try_recv() {
                if !new_path.is_empty() {
                    viewport_state.paths.push(new_path);
                    new_paths_added = true;
                }
            }

            // If new paths were added, fit the view to show all paths
            if new_paths_added {
                fit_map_to_paths(&mut viewport_state.map_memory, &viewport_state.paths);
                ctx.request_repaint();
            }

            // Only update position if it was explicitly changed from main window
            if target != viewport_state.last_set_position && !new_paths_added {
                viewport_state.map_memory.center_at(target);
                viewport_state.last_set_position = target;
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
                        let map = Map::new(None, &mut viewport_state.map_memory, target)
                            .with_layer(&mut viewport_state.tiles, 1.0);

                        map.show(ui, |ui, projector, _map_rect| {
                            // Draw all paths
                            for (idx, path) in viewport_state.paths.iter().enumerate() {
                                if path.len() >= 2 {
                                    // Use different colors for different paths
                                    let color = get_path_color(idx);
                                    draw_path(ui, &projector, path, color);
                                }
                            }

                            // Draw highlighted point if set
                            if let Some(highlight) = viewport_state.highlighted {
                                let pos2 = projector.project(highlight).to_pos2();
                                let painter = ui.painter();

                                // White dot with yellow outline
                                painter.circle_filled(pos2, 6.0, Color32::WHITE);
                                painter.circle_stroke(pos2, 8.0, Stroke::new(3.0, Color32::YELLOW));
                            }
                        });

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(format!("Lat: {:.6}, Lon: {:.6}", target.y(), target.x()));
                            ui.separator();
                            ui.label(format!("Paths: {}", viewport_state.paths.len()));
                        });
                    });
                },
            );

            // Close the viewport if user requested
            if !open {
                self.map_viewport_state = None;
            }
        }

        self.app.update(ctx, frame);
    }
}

impl AppWithMap {
    pub fn highlight_on_map(&mut self, lat: f64, lon: f64) {
        if let Some(viewport_state) = &mut self.map_viewport_state {
            let pos = Position::new(lon, lat);
            viewport_state.highlighted = Some(pos);
        }
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

    // Estimate zoom level (rough approximation)
    // Zoom level formula: each zoom level shows roughly half the area
    // At zoom 0, we see the whole world (~360 degrees)
    // At zoom 1, we see ~180 degrees, etc.
    let zoom = if max_span > 0.0 {
        // Add some padding (1.5x the span) and convert to zoom level
        let padded_span = max_span * 1.5;
        let zoom = (360.0 / padded_span).log2();
        zoom.clamp(2.0, 18.0)
    } else {
        10.0 // Default zoom if all points are the same
    };

    // Set the map center and zoom
    map_memory.center_at(center);
    map_memory.set_zoom(zoom).unwrap();
}

fn get_path_color(index: usize) -> Color32 {
    // Cycle through different colors for different paths
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

    // Convert geographic coordinates to screen coordinates
    let screen_points: Vec<Pos2> = path
        .iter()
        .map(|pos| projector.project(*pos).to_pos2())
        .collect();

    // Draw the path as a colored line
    let stroke = Stroke::new(3.0, color);

    for i in 0..screen_points.len() - 1 {
        painter.line_segment([screen_points[i], screen_points[i + 1]], stroke);
    }

    // Draw circles at each point
    for point in &screen_points {
        painter.circle_filled(*point, 4.0, color);
    }
}
