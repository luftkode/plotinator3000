use egui::{CentralPanel, Frame, ViewportBuilder, ViewportId};
use plotinator_log_if::prelude::GeoSpatialData;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};
use walkers::{HttpTiles, Map, MapMemory, Position, Tiles};

use crate::commander::MapUiCommander;

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
    /// Open the [MapViewPort]
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
                        visible: true,
                    });
                    self.fit_map_to_paths();
                }
                MapCommand::CursorPos(time_pos) => {
                    log::debug!("Got cursor time: {time_pos:.}");
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

                CentralPanel::default().frame(Frame::NONE).show(ctx, |ui| {
                    let map_state = self.map_tile_state.as_mut().unwrap();

                    if ui
                        .button(if map_state.is_satellite {
                            "Switch to OSM"
                        } else {
                            "Switch to Satellite"
                        })
                        .clicked()
                    {
                        let ctx_clone = ctx.clone();

                        // Load Mapbox token from env/compile-time
                        const MAPBOX_API_TOKEN_COMPILE_TIME_NAME: &str = "PLOTINATOR3000_MAPBOX_API";
                        const MAPBOX_API_TOKEN_FALLBACK: &str = "MAPBOX_ACCESS_TOKEN";
                        let mapbox_access_token = std::option_env!("PLOTINATOR3000_MAPBOX_API");

                        map_state.tiles = if map_state.is_satellite {
                            TilesKind::OSM(HttpTiles::new(
                                walkers::sources::OpenStreetMap,
                                ctx_clone,
                            ))
                        } else {
                            TilesKind::MapboxSatellite(HttpTiles::new(
                                walkers::sources::Mapbox {
                                    style: walkers::sources::MapboxStyle::Satellite,
                                    access_token: mapbox_access_token.to_owned().map(|s| s.to_string()).unwrap_or_else(||{
                                        log::error!("No mapbox api token in {MAPBOX_API_TOKEN_COMPILE_TIME_NAME} at compile time, falling back to {MAPBOX_API_TOKEN_FALLBACK}");
                                        std::env::var(MAPBOX_API_TOKEN_FALLBACK).expect("need mapbox api token").to_owned()

                                    }),
                                    high_resolution: true,
                                },
                                ctx_clone,
                            ))
                        };
                        map_state.is_satellite = !map_state.is_satellite;
                    }

                    let zoom_level = map_state.map_memory.zoom();
                    log::trace!("map zoom: {zoom_level:.1}");

                    let should_draw_height_labels = zoom_level > 18.;
                    let should_draw_heading_arrows = zoom_level >19.4;

                    let map = Map::new(
                        Some(map_state.tiles.as_mut()),
                        &mut map_state.map_memory,
                        self.map_data.center_position,
                    )
                    .double_click_to_zoom(true);

                    map.show(ui, |ui, projector, _map_rect| {
                        for (i, path_entry) in self.map_data.geo_data.iter().enumerate() {
                            if !path_entry.visible {
                                continue;
                            }

                            let is_hovered = self.hovered_path == Some(i);

                            draw::draw_path(
                                ui,
                                &projector,
                                &path_entry.data,
                                should_draw_heading_arrows,
                                should_draw_height_labels,
                            );

                            if is_hovered {
                                draw::highlight_whole_path(ui, &projector, &path_entry.data);
                            }
                        }


                        // Draw highlighted points based on cursor position
                        if let Some(cursor_time) = self.plot_time_cursor_pos {
                            draw::draw_cursor_highlights(
                                ui,
                                &projector,
                                &self.map_data.geo_data,
                                cursor_time,
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

                        for (i, path_entry) in self.map_data.geo_data.iter_mut().enumerate() {
                            let path = &path_entry.data;
                            let visible = &mut path_entry.visible;

                            ui.horizontal(|ui| {
                                // Toggle button
                                let indicator = if *visible {
                                    egui::RichText::new("⬤").color(path.color)
                                } else {
                                    egui::RichText::new("◯").color(path.color)
                                };
                                let button_resp = ui.button(indicator);

                                if button_resp.clicked() {
                                    *visible = !*visible;
                                }
                                let name_label_resp = ui.label(&path.name);

                                // Metadata flags
                                let mut meta = String::new();
                                if path.points.first().and_then(|p| p.speed).is_some() {
                                    meta.push_str("S ");
                                }
                                if path.points.first().and_then(|p| p.altitude).is_some() {
                                    meta.push_str("A ");
                                }
                                if path.points.first().and_then(|p| p.heading).is_some() {
                                    meta.push_str("H ");
                                }
                                let mut meta_label_hovered = false;
                                if !meta.is_empty() {
                                    meta_label_hovered = ui.label(format!("[{}]", meta.trim())).hovered();
                                }



                                // Track hover
                                if button_resp.hovered() || name_label_resp.hovered() || meta_label_hovered {
                                    self.hovered_path = Some(i);
                                }
                            });
                        }

                        // Reset hovered if nothing hovered this frame

                        if !ui.ui_contains_pointer() {
                            self.hovered_path = None;
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

pub enum TilesKind {
    OSM(HttpTiles),
    MapboxSatellite(HttpTiles),
}

impl AsMut<dyn Tiles> for TilesKind {
    fn as_mut(&mut self) -> &mut (dyn Tiles + 'static) {
        match self {
            TilesKind::OSM(tiles) => tiles,
            TilesKind::MapboxSatellite(tiles) => tiles,
        }
    }
}

impl AsRef<dyn Tiles> for TilesKind {
    fn as_ref(&self) -> &(dyn Tiles + 'static) {
        match self {
            TilesKind::OSM(tiles) => tiles,
            TilesKind::MapboxSatellite(tiles) => tiles,
        }
    }
}

pub struct MapTileState {
    map_memory: MapMemory,
    pub tiles: TilesKind,
    pub is_satellite: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PathEntry {
    pub data: GeoSpatialData,
    pub visible: bool,
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
    if geo_data.is_empty() {
        return None;
    }

    // Find bounding box
    let mut min_lat: Option<f64> = None;
    let mut max_lat: Option<f64> = None;
    let mut min_lon: Option<f64> = None;
    let mut max_lon: Option<f64> = None;

    for path in geo_data.iter().filter(|p| p.visible) {
        let gd = &path.data;
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
