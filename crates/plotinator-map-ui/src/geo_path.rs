use egui::{Color32, Pos2, Rect, vec2};
use plotinator_log_if::{
    prelude::{GeoPoint, PrimaryGeoSpatialData},
    rawplot::path_data::caching::CachedValues,
};
use plotinator_mqtt::data::listener::MqttGeoPoint;
use plotinator_ui_util::auto_terrain_safe_color;
use serde::{Deserialize, Serialize};
use walkers::Projector;

/// A trait for any type that represents a traversable geospatial path.
pub trait GeoPath {
    /// Provides a slice of the geographical points in the path.
    fn points(&self) -> &[GeoPoint];

    /// Returns `true` if the path is currently visible.
    fn is_visible(&self) -> bool;

    /// Returns the color assigned to this [`GeoPath`]
    fn color(&self) -> Color32;

    /// Get the latitude bounds (min, max)
    fn lat_bounds(&self) -> (f64, f64);

    /// Get the longitude bounds (min, max)
    fn lon_bounds(&self) -> (f64, f64);

    /// Get the speed bounds (min, max) if available
    fn speed_bounds(&self) -> (f64, f64);
}

impl GeoPath for MqttGeoPath {
    fn points(&self) -> &[GeoPoint] {
        &self.points
    }

    fn is_visible(&self) -> bool {
        self.settings.visible
    }

    fn color(&self) -> Color32 {
        self.color
    }

    fn lat_bounds(&self) -> (f64, f64) {
        self.boundary_values.lat_bounds()
    }

    fn lon_bounds(&self) -> (f64, f64) {
        self.boundary_values.lon_bounds()
    }

    fn speed_bounds(&self) -> (f64, f64) {
        self.boundary_values.speed_bounds()
    }
}

impl GeoPath for PathEntry {
    fn points(&self) -> &[GeoPoint] {
        &self.data.points
    }

    fn is_visible(&self) -> bool {
        self.settings.visible
    }

    fn color(&self) -> Color32 {
        self.data.color
    }

    fn lat_bounds(&self) -> (f64, f64) {
        self.data.lat_bounds()
    }

    /// Get the longitude bounds (min, max) if available
    fn lon_bounds(&self) -> (f64, f64) {
        self.data.lon_bounds()
    }

    /// Get the speed bounds (min, max) if available
    fn speed_bounds(&self) -> (f64, f64) {
        self.data.speed_bounds()
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PathEntry {
    pub data: PrimaryGeoSpatialData,
    pub settings: GeoPathSettings,
}

impl PathEntry {
    /// Name of the Primary Geospatial dataset
    pub fn name(&self) -> &str {
        &self.data.name
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct GeoPathSettings {
    pub visible: bool,
    pub show_heading: bool,  // if applicable
    pub show_altitude: bool, // if applicable
    pub show_speed: bool,    // if applicable
}

impl Default for GeoPathSettings {
    fn default() -> Self {
        Self {
            visible: true,
            show_heading: true,
            show_altitude: true,
            show_speed: true,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct MqttGeoPath {
    pub(crate) topic: String,
    pub(crate) points: Vec<GeoPoint>,
    pub(crate) settings: GeoPathSettings,
    pub boundary_values: CachedValues,
    pub color: Color32,
}

impl From<MqttGeoPoint> for MqttGeoPath {
    fn from(mqtt_point: MqttGeoPoint) -> Self {
        let MqttGeoPoint { topic, point } = mqtt_point;
        let boundary_values = CachedValues::compute(&[point]);
        Self {
            topic,
            points: vec![point],
            settings: GeoPathSettings::default(),
            boundary_values,
            color: auto_terrain_safe_color(),
        }
    }
}

impl MqttGeoPath {
    pub fn push(&mut self, point: GeoPoint) {
        self.boundary_values.update_from_point(&point);
        self.points.push(point);
    }
}

pub struct ClosestPoint {
    pub timestamp: f64,
    pub screen_pos: Pos2,
    pub distance_to_pointer: f32,
    pub path_color: Color32,
}

/// Finds the timestamp of the closest `GeoPoint` to a cursor position across multiple paths.
pub fn find_closest_point<'a>(
    paths: impl IntoIterator<Item = &'a (impl GeoPath + 'a)>,
    pointer_pos: Pos2,
    projector: &Projector,
) -> Option<ClosestPoint> {
    const SEARCH_RADIUS_PIXELS: f32 = 10.0;
    let search_radius_sq = SEARCH_RADIUS_PIXELS.powi(2);

    // Create a search box around the cursor in screen space
    let search_box = Rect::from_center_size(
        pointer_pos,
        vec2(SEARCH_RADIUS_PIXELS * 2.0, SEARCH_RADIUS_PIXELS * 2.0),
    );

    // Convert screen search box to lat/lon bounds
    // Project the corners to get approximate lat/lon bounds
    let top_left_geo = projector.unproject(search_box.left_top().to_vec2());
    let bottom_right_geo = projector.unproject(search_box.right_bottom().to_vec2());

    let (lat_min_search, lat_max_search) = if top_left_geo.y() < bottom_right_geo.y() {
        (top_left_geo.y(), bottom_right_geo.y())
    } else {
        (bottom_right_geo.y(), top_left_geo.y())
    };

    let (lon_min_search, lon_max_search) = if top_left_geo.x() < bottom_right_geo.x() {
        (top_left_geo.x(), bottom_right_geo.x())
    } else {
        (bottom_right_geo.x(), top_left_geo.x())
    };

    let mut closest: Option<ClosestPoint> = None;
    let mut min_dist_sq = search_radius_sq;

    for path in paths {
        if !path.is_visible() {
            continue;
        }

        // Early rejection: check if path's geographic bounds intersect search bounds
        let (path_lat_min, path_lat_max) = path.lat_bounds();
        let (path_lon_min, path_lon_max) = path.lon_bounds();

        // skip if no overlap
        if path_lat_max < lat_min_search
            || path_lat_min > lat_max_search
            || path_lon_max < lon_min_search
            || path_lon_min > lon_max_search
        {
            continue;
        }

        // Only project and check points from paths that overlap
        for point in path.points() {
            // Filter by geographic bounds first (cheap operation on f64)
            let geo_pos = point.position;
            if geo_pos.y() < lat_min_search
                || geo_pos.y() > lat_max_search
                || geo_pos.x() < lon_min_search
                || geo_pos.x() > lon_max_search
            {
                continue;
            }

            let screen_pos = projector.project(geo_pos).to_pos2();
            let dist_sq = pointer_pos.distance_sq(screen_pos);

            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                closest = Some(ClosestPoint {
                    timestamp: point.timestamp,
                    screen_pos,
                    distance_to_pointer: dist_sq,
                    path_color: path.color(),
                });
            }
        }
    }

    closest
}
