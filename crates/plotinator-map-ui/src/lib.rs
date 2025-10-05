use egui::{CentralPanel, Color32, Frame, RichText, Ui, ViewportBuilder, ViewportId};
use egui_phosphor::regular::{
    CHECK_CIRCLE, CHECK_SQUARE, CIRCLE, GLOBE, GLOBE_HEMISPHERE_WEST, SQUARE,
};
use plotinator_log_if::prelude::GeoSpatialData;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use walkers::{HttpTiles, Map, MapMemory, Position, Tiles};

use crate::{
    commander::MapUiCommander,
    draw::{DrawSettings, TelemetryLabelSettings},
};

/// Messages sent from main app to map viewport
pub enum MapCommand {
    AddGeoData(GeoSpatialData),
    /// Cursor position on the time axis
    CursorPos(f64),
    FitToAllPaths,
}

pub mod commander;
mod draw;

#[derive(Default, Deserialize, Serialize)]
pub struct MapViewPort {
    pub open: bool,
    map_data: MapData,
    // Cached map data (external), instantiated on first open, loaded on demand
    #[serde(skip)]
    map_tile_state: Option<MapTileState>,
    #[serde(skip)]
    cmd_recv: Option<Receiver<MapCommand>>,
    /// The time corresponding to the cursor position in the plot area
    #[serde(skip)]
    plot_time_cursor_pos: Option<f64>,
    #[serde(skip)]
    hovered_path: Option<usize>, // index of hovered path
}

impl MapViewPort {
    /// Open the [`MapViewPort`]
    ///
    /// if it's the first time it's opened, it will start loading map tiles and
    /// return a [Sender<MapCommand>] for interacting with the Map from other contexts
    pub fn open(&mut self, ctx: &egui::Context) -> Option<Sender<MapCommand>> {
        if self.map_tile_state.is_none() {
            egui_extras::install_image_loaders(ctx);

            let tiles =
                TilesKind::OSM(HttpTiles::new(walkers::sources::OpenStreetMap, ctx.clone()));
            let mut map_memory = MapMemory::default();
            map_memory.center_at(self.map_data.center_position);
            let _ = map_memory.set_zoom(self.map_data.zoom);

            self.map_tile_state = Some(MapTileState {
                map_memory,
                tiles,
                is_satellite: false,
            });
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
        let mut cursor_pos: Option<f64> = None;
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
                    self.map_data.geo_data.push(PathEntry {
                        data: geo_data,
                        settings: Default::default(),
                    });
                    self.fit_map_to_paths();
                }
                MapCommand::CursorPos(time_pos) => {
                    log::trace!("Got cursor time: {time_pos:.}");
                    cursor_pos = Some(time_pos);
                }
                MapCommand::FitToAllPaths => {
                    self.fit_map_to_paths();
                }
            }
        }
        if let Some(pos) = cursor_pos {
            self.plot_time_cursor_pos = Some(pos);
        }
    }

    fn fit_map_to_paths(&mut self) {
        if let Some((center, zoom)) = fit_map_to_paths(
            &mut self
                .map_tile_state
                .as_mut()
                .expect("unsound condition")
                .map_memory,
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
                .with_inner_size([800.0, 600.0])
                .with_drag_and_drop(false)
                .with_always_on_top(),
            |ctx, _class| {
                if ctx.input(|i| i.viewport().close_requested()) {
                    is_still_open = false;
                }

                self.poll_commands();

                CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                    self.show_map_panel(ui);
                });

                self.show_legend_window(ctx);
            },
        );

        // If the user requested to close the window, update the state.
        if !is_still_open {
            self.close();
        }
    }

    /// Renders the main map panel and all geographical data on it.
    fn show_map_panel(&mut self, ui: &mut Ui) {
        let map_state = self
            .map_tile_state
            .as_mut()
            .expect("map_tile_state is required but not initialized");

        let zoom_level = map_state.map_memory.zoom();

        let map = Map::new(
            Some(map_state.tiles.as_mut()),
            &mut map_state.map_memory,
            self.map_data.center_position,
        )
        .double_click_to_zoom(true);

        map.show(ui, |ui, projector, _map_rect| {
            for (i, path) in self.map_data.geo_data.iter().enumerate() {
                if !path.settings.visible {
                    continue;
                }

                let is_hovered = self.hovered_path == Some(i);

                let draw_settings = DrawSettings {
                    draw_heading_arrows: zoom_level > 18.0 && path.settings.show_heading,
                    telemetry_label: TelemetryLabelSettings {
                        draw: zoom_level > 19.4,
                        with_speed: path.settings.show_speed,
                        with_altitude: path.settings.show_altitude,
                    },
                };

                draw::draw_path(ui, projector, &path.data, &draw_settings);

                if is_hovered {
                    draw::highlight_whole_path(ui.painter(), projector, &path.data);
                }
            }

            if let Some(cursor_time) = self.plot_time_cursor_pos {
                draw::draw_cursor_highlights(
                    ui.painter(),
                    projector,
                    &self.map_data.geo_data,
                    cursor_time,
                );
            }
        });
    }

    /// Renders the legend window with map controls and path information.
    fn show_legend_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Legend")
            .title_bar(true)
            .resizable(true)
            .default_pos(egui::pos2(0.0, 10.0))
            .default_size([200.0, 150.0])
            .show(ctx, |ui| {
                let map_state = self
                    .map_tile_state
                    .as_mut()
                    .expect("map_tile_state is required but not initialized");

                let button_text = if map_state.is_satellite {
                    RichText::new(format!("{GLOBE} Open Street Map {GLOBE}")).strong()
                } else {
                    RichText::new(format!(
                        "{GLOBE_HEMISPHERE_WEST} Satellite {GLOBE_HEMISPHERE_WEST}"
                    ))
                    .strong()
                };

                if ui.button(button_text).clicked() {
                    self.toggle_map_style(ctx);
                }

                if self.map_data.geo_data.is_empty() {
                    ui.label("No paths loaded");
                } else {
                    ui.separator();
                    self.show_legend_grid(ui);
                }

                // Reset hovered state if the mouse leaves the legend window
                if !ui.ui_contains_pointer() {
                    self.hovered_path = None;
                }
            });
    }

    /// Toggles the map tile source between OpenStreetMap and Mapbox Satellite.
    fn toggle_map_style(&mut self, ctx: &egui::Context) {
        let map_state = self
            .map_tile_state
            .as_mut()
            .expect("map_tile_state is required but not initialized");
        let ctx_clone = ctx.clone();

        if map_state.is_satellite {
            map_state.tiles =
                TilesKind::OSM(HttpTiles::new(walkers::sources::OpenStreetMap, ctx_clone));
        } else {
            const MAPBOX_API_TOKEN_COMPILE_TIME_NAME: &str = "PLOTINATOR3000_MAPBOX_API";
            const MAPBOX_API_TOKEN_FALLBACK: &str = "PLOTINATOR3000_MAPBOX_API_LOCAL";

            let access_token = option_env!("PLOTINATOR3000_MAPBOX_API").map_or_else(
                || {
                    log::error!("No mapbox api token in {MAPBOX_API_TOKEN_COMPILE_TIME_NAME} at compile time, falling back to {MAPBOX_API_TOKEN_FALLBACK}");
                    std::env::var(MAPBOX_API_TOKEN_FALLBACK).expect("need mapbox api token")
                },
                |s| s.to_owned(),
            );

            map_state.tiles = TilesKind::MapboxSatellite(HttpTiles::new(
                walkers::sources::Mapbox {
                    style: walkers::sources::MapboxStyle::Satellite,
                    access_token,
                    high_resolution: true,
                },
                ctx_clone,
            ));
        }
        map_state.is_satellite = !map_state.is_satellite;
    }

    fn show_legend_grid(&mut self, ui: &mut Ui) {
        egui::Grid::new("legend_grid").striped(true).show(ui, |ui| {
            // Column headers
            ui.label(""); // empty cell for toggle + name
            ui.label("vel");
            ui.label("alt");
            ui.label("hdg");
            ui.end_row();

            for (i, path_entry) in self.map_data.geo_data.iter_mut().enumerate() {
                let path = &path_entry.data;

                let mut path_ui_hovered = false;
                ui.horizontal(|ui| {
                    // Visibility toggle
                    let indicator = if path_entry.settings.visible {
                        RichText::new(CHECK_SQUARE).color(path.color).weak()
                    } else {
                        RichText::new(SQUARE).color(path.color).strong()
                    };
                    if ui.button(indicator).clicked() {
                        path_entry.settings.visible = !path_entry.settings.visible;
                    }
                    ui.label(RichText::new(&path.name).strong());

                    if ui.ui_contains_pointer() {
                        path_ui_hovered = true;
                    }
                });

                let first_point = path.points.first();
                let mut attr_indicator_label = |has_attr: bool, show_attr: &mut bool| {
                    let has_attr_text = if has_attr {
                        if *show_attr {
                            RichText::new(CHECK_CIRCLE).color(Color32::GREEN)
                        } else {
                            RichText::new(CIRCLE).color(Color32::GREEN).weak()
                        }
                    } else {
                        RichText::new(CIRCLE).weak()
                    };
                    let resp = ui.button(has_attr_text);
                    if resp.clicked() {
                        *show_attr = !*show_attr;
                    }
                    if resp.hovered() {
                        path_ui_hovered = true;
                    }
                };

                // Velocity column
                let has_speed = first_point.and_then(|p| p.speed).is_some();
                attr_indicator_label(has_speed, &mut path_entry.settings.show_speed);

                // Altitude column
                let has_alt = first_point.and_then(|p| p.altitude).is_some();
                attr_indicator_label(has_alt, &mut path_entry.settings.show_altitude);

                // Heading column
                let has_heading = first_point.and_then(|p| p.heading).is_some();
                attr_indicator_label(has_heading, &mut path_entry.settings.show_heading);
                ui.end_row();

                // Hover highlighting
                if path_ui_hovered {
                    self.hovered_path = Some(i);
                }
            }
        });
    }
}

pub enum TilesKind {
    OSM(HttpTiles),
    MapboxSatellite(HttpTiles),
}

impl AsMut<dyn Tiles> for TilesKind {
    fn as_mut(&mut self) -> &mut (dyn Tiles + 'static) {
        match self {
            Self::OSM(tiles) | Self::MapboxSatellite(tiles) => tiles,
        }
    }
}

impl AsRef<dyn Tiles> for TilesKind {
    fn as_ref(&self) -> &(dyn Tiles + 'static) {
        match self {
            Self::OSM(tiles) | Self::MapboxSatellite(tiles) => tiles,
        }
    }
}

pub struct MapTileState {
    map_memory: MapMemory,
    pub tiles: TilesKind,
    pub is_satellite: bool,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct PathEntrySettings {
    pub visible: bool,
    pub show_heading: bool,  // if applicable
    pub show_altitude: bool, // if applicable
    pub show_speed: bool,    // if applicable
}

impl Default for PathEntrySettings {
    fn default() -> Self {
        Self {
            visible: true,
            show_heading: true,
            show_altitude: true,
            show_speed: true,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PathEntry {
    pub data: GeoSpatialData,
    pub settings: PathEntrySettings,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MapData {
    pub geo_data: Vec<PathEntry>,
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

fn fit_map_to_paths(map_memory: &mut MapMemory, geo_data: &[PathEntry]) -> Option<(Position, f64)> {
    let bounds = calculate_bounding_box(geo_data)?;

    let center = Position::new(bounds.center_lon(), bounds.center_lat());
    let zoom = bounds.zoom_level_to_fit_all();

    map_memory.center_at(center);
    let _ = map_memory.set_zoom(zoom);

    Some((center, zoom))
}

struct BoundingBox {
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
}

impl BoundingBox {
    fn center_lat(&self) -> f64 {
        (self.min_lat + self.max_lat) / 2.0
    }

    fn center_lon(&self) -> f64 {
        (self.min_lon + self.max_lon) / 2.0
    }

    fn lat_span(&self) -> f64 {
        self.max_lat - self.min_lat
    }

    fn lon_span(&self) -> f64 {
        self.max_lon - self.min_lon
    }

    fn zoom_level_to_fit_all(&self) -> f64 {
        let max_span = self.lat_span().max(self.lon_span());
        if max_span > 0.0 {
            let padded_span = max_span * 1.5;
            let zoom = (360.0 / padded_span).log2();
            zoom.clamp(2.0, 18.0)
        } else {
            10.0
        }
    }
}

fn calculate_bounding_box(geo_data: &[PathEntry]) -> Option<BoundingBox> {
    let visible_paths: Vec<&PathEntry> = geo_data.iter().filter(|p| p.settings.visible).collect();

    if visible_paths.is_empty() {
        return None;
    }

    let mut min_lat = f64::INFINITY;
    let mut max_lat = f64::NEG_INFINITY;
    let mut min_lon = f64::INFINITY;
    let mut max_lon = f64::NEG_INFINITY;

    for path in visible_paths {
        let gd = &path.data;
        let (tmp_min_lat, tmp_max_lat) = gd.lat_bounds();
        let (tmp_min_lon, tmp_max_lon) = gd.lon_bounds();

        log::debug!("{} - Lat bounds: [{tmp_min_lat}:{tmp_max_lat}]", gd.name);
        log::debug!("{} - Lon bounds: [{tmp_min_lon}:{tmp_max_lon}]", gd.name);

        min_lat = min_lat.min(tmp_min_lat);
        max_lat = max_lat.max(tmp_max_lat);
        min_lon = min_lon.min(tmp_min_lon);
        max_lon = max_lon.max(tmp_max_lon);
    }

    Some(BoundingBox {
        min_lat,
        max_lat,
        min_lon,
        max_lon,
    })
}
