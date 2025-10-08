use serde::{Deserialize, Serialize};
use walkers::{HttpTiles, MapMemory, Position, Tiles};

use crate::{MqttGeoPath, PathEntry};

/// Persisted state of the map, meaning tiles cache, geospatial data, centering etc.
#[derive(Default, Deserialize, Serialize)]
pub struct MapState {
    pub data: MapData,
    /// Cached map data (external), instantiated on first open, loaded on demand
    #[serde(skip)]
    pub tile_state: Option<MapTileState>,
}

impl MapState {
    pub fn init(&mut self, ctx: egui::Context) {
        debug_assert!(self.tile_state.is_none(), "double init");
        let tiles = TilesKind::OpenStreetMap(HttpTiles::new(walkers::sources::OpenStreetMap, ctx));
        let mut map_memory = MapMemory::default();
        map_memory.center_at(self.data.center_position);
        map_memory
            .set_zoom(self.data.zoom)
            .expect("init: invalid zoom setting");

        let map_tile_state = MapTileState {
            map_memory,
            tiles,
            is_satellite: false,
        };

        self.tile_state = Some(map_tile_state);
    }

    /// Toggles the map tile source between OpenStreetMap and Mapbox Satellite.
    pub(crate) fn toggle_map_style(&mut self, ctx: egui::Context) {
        let map_state = self
            .tile_state
            .as_mut()
            .expect("map_tile_state is required but not initialized");

        if map_state.is_satellite {
            map_state.tiles =
                TilesKind::OpenStreetMap(HttpTiles::new(walkers::sources::OpenStreetMap, ctx));
        } else {
            map_state.tiles = TilesKind::MapboxSatellite(HttpTiles::new(
                walkers::sources::Mapbox {
                    style: walkers::sources::MapboxStyle::Satellite,
                    access_token: get_mapbox_api_token(),
                    high_resolution: true,
                },
                ctx,
            ));
        }
        map_state.is_satellite = !map_state.is_satellite;
    }

    pub fn data(&self) -> &MapData {
        &self.data
    }

    pub fn zoom_to_fit(&mut self, geo_data: &[PathEntry], mqtt_geo_data: &[MqttGeoPath]) {
        let Some(bounds) = calculate_bounding_box(geo_data, mqtt_geo_data) else {
            log::warn!("Failed to calculate bounding box");
            return;
        };

        let center = Position::new(bounds.center_lon(), bounds.center_lat());
        let zoom = bounds.zoom_level_to_fit_all();
        log::info!("Zoom to fit level: {zoom:.1}");

        debug_assert!(
            self.tile_state.is_some(),
            "Attempted to zoom to fit with uninitialized map memory"
        );
        let Some(tile_state) = &mut self.tile_state else {
            log::error!("Attempt to zoom to fit with uninitialized tiles");
            return;
        };
        tile_state.map_memory.center_at(center);
        if let Err(e) = tile_state.map_memory.set_zoom(zoom) {
            log::error!("failed setting map zoom: {e}");
            debug_assert!(false, "failed setting map zoom {e}");
        }

        self.data.center_position = center;
        self.data.zoom = zoom;
    }

    pub(crate) fn tile_state_as_mut(&mut self) -> Option<&mut MapTileState> {
        self.tile_state.as_mut()
    }
}

pub enum TilesKind {
    OpenStreetMap(HttpTiles),
    MapboxSatellite(HttpTiles),
}

impl AsMut<dyn Tiles> for TilesKind {
    fn as_mut(&mut self) -> &mut (dyn Tiles + 'static) {
        match self {
            Self::OpenStreetMap(tiles) | Self::MapboxSatellite(tiles) => tiles,
        }
    }
}

impl AsRef<dyn Tiles> for TilesKind {
    fn as_ref(&self) -> &(dyn Tiles + 'static) {
        match self {
            Self::OpenStreetMap(tiles) | Self::MapboxSatellite(tiles) => tiles,
        }
    }
}

pub struct MapTileState {
    pub(crate) map_memory: MapMemory,
    pub tiles: TilesKind,
    pub is_satellite: bool,
}

impl MapTileState {
    pub(crate) fn zoom_level(&self) -> f64 {
        self.map_memory.zoom()
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MapData {
    pub highlighted: Option<Position>,
    pub center_position: Position,
    pub zoom: f64,
}

impl Default for MapData {
    fn default() -> Self {
        Self {
            highlighted: None,
            center_position: Position::new(-0.1278, 51.5074), // London (lon, lat)
            zoom: 10.0,
        }
    }
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
            16.0
        }
    }
}

fn calculate_bounding_box(
    geo_data: &[PathEntry],
    mqtt_geo_data: &[MqttGeoPath],
) -> Option<BoundingBox> {
    let visible_paths: Vec<&PathEntry> = geo_data.iter().filter(|p| p.settings.visible).collect();
    let visible_mqtt_paths: Vec<&MqttGeoPath> = mqtt_geo_data
        .iter()
        .filter(|p| p.settings.visible)
        .collect();

    if visible_paths.is_empty() && visible_mqtt_paths.is_empty() {
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
    for mqtt_path in visible_mqtt_paths {
        let (tmp_min_lat, tmp_max_lat) = mqtt_path.lat_bounds();
        let (tmp_min_lon, tmp_max_lon) = mqtt_path.lon_bounds();

        log::debug!(
            "{} - Lat bounds: [{tmp_min_lat}:{tmp_max_lat}]",
            mqtt_path.topic
        );
        log::debug!(
            "{} - Lon bounds: [{tmp_min_lon}:{tmp_max_lon}]",
            mqtt_path.topic
        );

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

const MAPBOX_API_TOKEN_COMPILE_TIME_NAME: &str = "PLOTINATOR3000_MAPBOX_API";
const MAPBOX_API_TOKEN_RUNTIME_LOCAL: &str = "PLOTINATOR3000_MAPBOX_API_LOCAL";
// Get a local runtime API token or fallback to the compile-time included public token
fn get_mapbox_api_token() -> String {
    std::env::var(MAPBOX_API_TOKEN_RUNTIME_LOCAL).ok().map_or_else(|| {
        log::info!("No local mapbox API token in {MAPBOX_API_TOKEN_RUNTIME_LOCAL}, falling back to compile-time token {MAPBOX_API_TOKEN_COMPILE_TIME_NAME}");
        #[allow(clippy::option_env_unwrap, reason = "that way every dev environment doesn't need an API token for development unrelated to map box")]
        option_env!("PLOTINATOR3000_MAPBOX_API").expect("no compile-time mapbox API token").to_owned()
    }, |s| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "meant to prove that CI sets a mapbox API token"]
    fn test_get_mapbox_api_token() {
        let api_token_is_not_empty = !get_mapbox_api_token().is_empty();
        debug_assert!(
            api_token_is_not_empty,
            "expected CI to set the map box API token in the environment variable: {MAPBOX_API_TOKEN_COMPILE_TIME_NAME}"
        );
    }
}
