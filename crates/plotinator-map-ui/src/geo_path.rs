use egui::Color32;
use plotinator_log_if::{
    prelude::{GeoPoint, PrimaryGeoSpatialData},
    rawplot::path_data::caching::CachedValues,
};
use plotinator_mqtt::data::listener::MqttGeoPoint;
use plotinator_ui_util::auto_terrain_safe_color;
use serde::{Deserialize, Serialize};

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

    /// Get the latitude bounds (min, max)
    pub fn lat_bounds(&self) -> (f64, f64) {
        self.data.lat_bounds()
    }

    /// Get the longitude bounds (min, max) if available
    pub fn lon_bounds(&self) -> (f64, f64) {
        self.data.lon_bounds()
    }

    /// Get the speed bounds (min, max) if available
    pub fn speed_bounds(&self) -> (f64, f64) {
        self.data.speed_bounds()
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

    /// Get the latitude bounds (min, max)
    pub fn lat_bounds(&self) -> (f64, f64) {
        self.boundary_values.lat_bounds()
    }

    /// Get the longitude bounds (min, max) if available
    pub fn lon_bounds(&self) -> (f64, f64) {
        self.boundary_values.lon_bounds()
    }

    /// Get the speed bounds (min, max) if available
    pub fn speed_bounds(&self) -> (f64, f64) {
        self.boundary_values.speed_bounds()
    }
}
