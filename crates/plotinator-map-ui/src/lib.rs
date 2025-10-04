use egui::{CentralPanel, Color32, Frame, Pos2, Stroke, Vec2, ViewportBuilder, ViewportId};
use plotinator_log_if::prelude::{GeoPoint, GeoSpatialData};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use walkers::{HttpTiles, Map, MapMemory, Position, Projector};

use crate::commander::MapUiCommander;

/// Messages sent from main app to map viewport
pub enum MapCommand {
    AddGeoData(GeoSpatialData),
    /// Cursor position on the time axis
    CursorPos(f64),
    FitToAllPaths,
}

pub mod commander;

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
                    if let Some(first_point) = geo_data.points.first() {
                        let has_speed = first_point.speed.is_some();
                        let has_altitude = first_point.altitude.is_some();
                        let has_heading = first_point.heading.is_some();
                        log::info!(
                            "Received geo data {}, points include speed={has_speed}, altitude={has_altitude}, heading={has_heading}",
                            geo_data.name
                        );
                    } else {
                        log::info!("Received basic geo data {}", geo_data.name);
                    }
                    debug_assert!(
                        geo_data.points.iter().all(|p| !p.timestamp.is_nan()
                            && !p.position.x().is_nan()
                            && !p.position.y().is_nan()
                            && !p
                                .altitude
                                .is_some_and(|a| a.is_nan() && p.speed.is_some_and(|s| s.is_nan()))
                            && !p.heading.is_some_and(|h| h.is_nan())),
                        "GeoSpatialData with NaN values: {}",
                        geo_data.name
                    );
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
                let zoom_level = map_memory.zoom();
                log::trace!("map zoom: {zoom_level:.1}");
                let should_draw_height_labels = zoom_level > 18.;
                let should_draw_heading_arrows = zoom_level > 21.;
                CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                    let map = Map::new(Some(tiles), map_memory, self.map_data.center_position)
                        .double_click_to_zoom(true);

                    map.show(ui, |ui, projector, _map_rect| {
                        for geo_data in &self.map_data.geo_data {
                            draw_path(
                                ui,
                                &projector,
                                geo_data,
                                should_draw_heading_arrows,
                                should_draw_height_labels,
                            );
                        }
                    });
                });

                egui::Window::new("Legend")
                    .title_bar(true)
                    .resizable(true)
                    .default_pos(egui::pos2(10.0, 10.0))
                    .default_size([200.0, 150.0])
                    .show(ctx, |ui| {
                        if self.map_data.geo_data.is_empty() {
                            ui.label("No paths loaded");
                            return;
                        }

                        for path in &self.map_data.geo_data {
                            // First point determines metadata availability
                            let has_heading = path.points.first().and_then(|p| p.heading).is_some();
                            let has_speed = path.points.first().and_then(|p| p.speed).is_some();
                            let has_altitude =
                                path.points.first().and_then(|p| p.altitude).is_some();

                            ui.horizontal(|ui| {
                                // Color indicator
                                ui.colored_label(path.color, "⬤");

                                // Path name
                                ui.label(&path.name);

                                // Metadata flags
                                let mut meta = String::new();
                                if has_speed {
                                    meta.push_str("S ");
                                }
                                if has_altitude {
                                    meta.push_str("A ");
                                }
                                if has_heading {
                                    meta.push_str("H ");
                                }
                                if !meta.is_empty() {
                                    ui.label(format!("[{}]", meta.trim()));
                                }
                            });
                        }
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
        log::debug!("{} - Lat bounds: [{tmp_min_lat}:{tmp_max_lat}]", gd.name);
        log::debug!("{} - Lon bounds: [{tmp_min_lon}:{tmp_max_lon}]", gd.name);
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

fn draw_path(
    ui: &mut egui::Ui,
    projector: &Projector,
    path: &GeoSpatialData,
    should_draw_heading_arrows: bool,
    should_draw_height_labels: bool,
) {
    if path.points.len() < 2 {
        return;
    }

    let painter = ui.painter();
    let path_color = path.color;

    // We need the full GeoPoint data at each screen position to access speed and heading.
    let screen_points: Vec<(Pos2, &GeoPoint)> = path
        .points
        .iter()
        .map(|p| (projector.project(p.position).to_pos2(), p))
        .collect();

    // Draw the path as a colored line with altitude-based opacity
    const MAX_ALTITUDE: f64 = 1000.0;

    for window in screen_points.windows(2) {
        // Use the altitude from the first point of the segment
        let altitude = window[0].1.altitude.unwrap_or(0.0);
        let opacity = (altitude / MAX_ALTITUDE).clamp(0.0, 1.0);

        // Scale the alpha channel of the path color based on altitude
        let alpha = (255.0 * opacity) as u8;
        let segment_color = Color32::from_rgba_unmultiplied(
            path_color.r(),
            path_color.g(),
            path_color.b(),
            alpha.max(20), // Minimum alpha of 20 to ensure visibility
        );

        let stroke = Stroke::new(3.0, segment_color);
        painter.line_segment([window[0].0, window[1].0], stroke);
    }

    // Get the speed range for the entire path to normalize arrow lengths.
    let speed_range = path.speed_bounds();

    // Draw circles at each point
    for (point_pos, _geo_point) in &screen_points {
        painter.circle_stroke(*point_pos, 2.0, Stroke::new(1.0, path_color));
    }

    // Draw heading arrows with distance-based filtering
    if should_draw_heading_arrows {
        draw_heading_arrows(painter, &screen_points, path_color, speed_range);
    }

    if should_draw_height_labels {
        draw_altitude_labels(painter, &screen_points);
    }

    // Draw start marker (filled black circle)
    if let Some((start_pos, _)) = screen_points.first() {
        draw_start_marker(painter, *start_pos);
    }

    // Draw end marker (black cross)
    if let Some((end_pos, _)) = screen_points.last() {
        draw_end_marker(painter, *end_pos);
    }
}

fn draw_heading_arrows(
    painter: &egui::Painter,
    screen_points: &[(Pos2, &GeoPoint)],
    path_color: Color32,
    speed_range: (f64, f64),
) {
    const MIN_ARROW_DISTANCE: f32 = 40.0; // Minimum pixels between arrows

    let mut last_arrow_pos: Option<Pos2> = None;

    for (point_pos, geo_point) in screen_points.iter() {
        // Skip if no heading data
        if geo_point.heading.is_none() {
            continue;
        }

        // Check distance from last drawn arrow
        let should_draw = if let Some(last_pos) = last_arrow_pos {
            let distance = point_pos.distance(last_pos);
            distance >= MIN_ARROW_DISTANCE
        } else {
            true // Always draw the first arrow
        };

        if should_draw {
            draw_heading_arrow(painter, *point_pos, geo_point, path_color, speed_range);
            last_arrow_pos = Some(*point_pos);
        }
    }
}

fn draw_heading_arrow(
    painter: &egui::Painter,
    center: Pos2,
    geo_point: &plotinator_log_if::prelude::GeoPoint,
    _path_color: Color32, // No longer needed, but kept for signature consistency
    speed_range: (f64, f64),
) {
    let Some(heading_deg) = geo_point.heading else {
        return;
    };

    const MIN_ARROW_LENGTH: f32 = 4.0;
    const MAX_ARROW_LENGTH: f32 = 30.0;
    const DEFAULT_ARROW_LENGTH: f32 = 12.0;

    let arrow_length = if let Some(speed) = geo_point.speed {
        let (min_speed, max_speed) = speed_range;
        if max_speed > min_speed {
            let speed_ratio =
                ((speed - min_speed) / (max_speed - min_speed)).clamp(0.0, 1.0) as f32;
            MIN_ARROW_LENGTH + speed_ratio * (MAX_ARROW_LENGTH - MIN_ARROW_LENGTH)
        } else {
            DEFAULT_ARROW_LENGTH
        }
    } else {
        DEFAULT_ARROW_LENGTH
    };

    // --- Correct Angle and Geometry Calculation ---
    // 0° North -> Up, 90° East -> Right
    let angle_rad = (90.0 - heading_deg).to_radians() as f32;
    let dir = Vec2::new(angle_rad.cos(), -angle_rad.sin());

    let tip = center + dir * arrow_length;

    // Calculate the two barbs for the arrowhead by rotating the backward vector
    let barb_length = arrow_length * 0.4;
    let barb_angle = 25.0_f32.to_radians();
    let back_dir = -dir;

    let rot = egui::emath::Rot2::from_angle(barb_angle);
    let barb1 = tip + (rot * back_dir) * barb_length;
    let barb2 = tip + (rot.inverse() * back_dir) * barb_length;

    let outline_color = Color32::BLACK;
    let outline_stroke = Stroke::new(1.5, outline_color);

    painter.line_segment([center, tip], outline_stroke);
    painter.line_segment([tip, barb1], outline_stroke);
    painter.line_segment([tip, barb2], outline_stroke);
}

fn draw_start_marker(painter: &egui::Painter, center: Pos2) {
    const MARKER_RADIUS: f32 = 6.0;

    // Draw white outline for better visibility
    painter.circle_filled(center, MARKER_RADIUS + 1.0, Color32::WHITE);
    // Draw black filled circle
    painter.circle_filled(center, MARKER_RADIUS, Color32::BLACK);
}

fn draw_end_marker(painter: &egui::Painter, center: Pos2) {
    const CROSS_SIZE: f32 = 8.0;
    const CROSS_THICKNESS: f32 = 2.5;

    let stroke = Stroke::new(CROSS_THICKNESS, Color32::BLACK);
    let outline_stroke = Stroke::new(CROSS_THICKNESS + 1.0, Color32::WHITE);

    // Draw white outline for better visibility
    painter.line_segment(
        [
            center + Vec2::new(-CROSS_SIZE, -CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, CROSS_SIZE),
        ],
        outline_stroke,
    );
    painter.line_segment(
        [
            center + Vec2::new(-CROSS_SIZE, CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, -CROSS_SIZE),
        ],
        outline_stroke,
    );

    // Draw black cross
    painter.line_segment(
        [
            center + Vec2::new(-CROSS_SIZE, -CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, CROSS_SIZE),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + Vec2::new(-CROSS_SIZE, CROSS_SIZE),
            center + Vec2::new(CROSS_SIZE, -CROSS_SIZE),
        ],
        stroke,
    );
}

fn draw_altitude_labels(painter: &egui::Painter, screen_points: &[(Pos2, &GeoPoint)]) {
    const MIN_LABEL_DISTANCE: f32 = 60.0; // Minimum pixels between labels

    let mut last_label_pos: Option<Pos2> = None;

    for (i, (point_pos, geo_point)) in screen_points.iter().enumerate() {
        // Skip if no altitude data
        let Some(altitude) = geo_point.altitude else {
            continue;
        };

        // Check every 10th point as a candidate
        if i % 10 != 0 {
            continue;
        }

        // Check distance from last drawn label
        let should_draw = if let Some(last_pos) = last_label_pos {
            let distance = point_pos.distance(last_pos);
            distance >= MIN_LABEL_DISTANCE
        } else {
            true // Always draw the first label
        };

        if should_draw {
            draw_altitude_label(painter, *point_pos, altitude);
            last_label_pos = Some(*point_pos);
        }
    }
}

fn draw_altitude_label(painter: &egui::Painter, point: Pos2, altitude: f64) {
    let text = format!("{:.0}m", altitude);
    let font_id = egui::FontId::proportional(11.0);
    let text_color = Color32::BLACK;
    let bg_color = Color32::from_rgba_unmultiplied(255, 255, 255, 200);

    // Offset the text slightly above and to the right of the point
    let text_pos = point + Vec2::new(5.0, -8.0);

    // Get text dimensions for background
    let galley = painter.layout_no_wrap(text.clone(), font_id.clone(), text_color);
    let text_rect = egui::Rect::from_min_size(text_pos, galley.size());
    let padded_rect = text_rect.expand(2.0);

    // Draw background
    painter.rect_filled(padded_rect, 2.0, bg_color);

    // Draw text
    painter.text(text_pos, egui::Align2::LEFT_TOP, text, font_id, text_color);
}
