use serde::{Deserialize, Serialize};

use crate::{
    prelude::{GeoAltitude, GeoPoint},
    rawplot::path_data::Altitude,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CachedValues {
    lat_min_max: (f64, f64),
    lon_min_max: (f64, f64),
    speed_min_max: (f64, f64),
    altitude_min_max: (f64, f64),
}

impl CachedValues {
    fn valid_altitude(altitude: f64) -> bool {
        altitude.is_sign_positive() && altitude < 3000.
    }

    pub fn compute(points: &[GeoPoint]) -> Self {
        if points.is_empty() {
            return Self::default();
        }

        // Compute lat/lon bounds
        let mut lat_min = f64::INFINITY;
        let mut lat_max = f64::NEG_INFINITY;
        let mut lon_min = f64::INFINITY;
        let mut lon_max = f64::NEG_INFINITY;

        // Compute speed bounds
        let mut speed_min = f64::INFINITY;
        let mut speed_max = f64::NEG_INFINITY;

        // Compute altitude bounds
        let mut altitude_min = f64::INFINITY;
        let mut altitude_max = f64::NEG_INFINITY;

        for point in points {
            let lat = point.position.y();
            let lon = point.position.x();
            lat_min = lat_min.min(lat);
            lat_max = lat_max.max(lat);
            lon_min = lon_min.min(lon);
            lon_max = lon_max.max(lon);

            if let Some(speed) = point.speed {
                speed_min = speed_min.min(speed);
                speed_max = speed_max.max(speed);
            }
            for alt in &point.altitude {
                if let Altitude::Valid(v) = alt.val() {
                    altitude_min = altitude_min.min(v);
                    altitude_max = altitude_max.max(v);
                }
            }
        }

        let lat_min_max = (lat_min, lat_max);
        let lon_min_max = (lon_min, lon_max);

        // If no points had speed data, speed_max will still be NEG_INFINITY.
        // In this case, we can set a default range like (0.0, 0.0).
        let speed_min_max = if speed_max.is_finite() {
            (speed_min, speed_max)
        } else {
            (0.0, 0.0)
        };

        // If no points had altitude data, altitude_max will still be NEG_INFINITY.
        // In this case, we can set a default range like (0.0, 0.0).
        let altitude_min_max = if altitude_max.is_finite() {
            (altitude_min, altitude_max)
        } else {
            (0.0, 0.0)
        };

        Self {
            lat_min_max,
            lon_min_max,
            speed_min_max,
            altitude_min_max,
        }
    }

    /// Get the latitude bounds (min, max)
    pub fn lat_bounds(&self) -> (f64, f64) {
        self.lat_min_max
    }

    /// Get the longitude bounds (min, max) if available
    pub fn lon_bounds(&self) -> (f64, f64) {
        self.lon_min_max
    }

    /// Get the speed bounds (min, max) if available
    pub fn speed_bounds(&self) -> (f64, f64) {
        self.speed_min_max
    }

    /// Update latitude bounds with a new latitude value
    pub fn update_lat(&mut self, lat: f64) {
        self.lat_min_max.0 = self.lat_min_max.0.min(lat);
        self.lat_min_max.1 = self.lat_min_max.1.max(lat);
    }

    /// Update longitude bounds with a new longitude value
    pub fn update_lon(&mut self, lon: f64) {
        self.lon_min_max.0 = self.lon_min_max.0.min(lon);
        self.lon_min_max.1 = self.lon_min_max.1.max(lon);
    }

    /// Update speed bounds with a new speed value (if present)
    pub fn update_speed(&mut self, speed: Option<f64>) {
        if let Some(speed) = speed {
            // Handle the case where cache is still default (0.0, 0.0)
            if self.speed_min_max == (0.0, 0.0) {
                self.speed_min_max = (speed, speed);
            } else {
                self.speed_min_max.0 = self.speed_min_max.0.min(speed);
                self.speed_min_max.1 = self.speed_min_max.1.max(speed);
            }
        }
    }

    /// Update altitude bounds with a new altitude value (if present and valid)
    pub fn update_altitude(&mut self, altitude: Option<f64>) {
        if let Some(alt) = altitude
            && Self::valid_altitude(alt)
        {
            // Handle the case where cache is still default (0.0, 0.0)
            if self.altitude_min_max == (0.0, 0.0) {
                self.altitude_min_max = (alt, alt);
            } else {
                self.altitude_min_max.0 = self.altitude_min_max.0.min(alt);
                self.altitude_min_max.1 = self.altitude_min_max.1.max(alt);
            }
        }
    }

    /// Convenience method to update all bounds from a single [`GeoPoint`]
    pub fn update_from_point(&mut self, point: &GeoPoint) {
        let lat = point.position.y();
        let lon = point.position.x();

        self.update_lat(lat);
        self.update_lon(lon);
        self.update_speed(point.speed);

        for alt in &point.altitude {
            match alt {
                GeoAltitude::Gnss(alt) | GeoAltitude::Laser(alt) => match alt {
                    Altitude::Valid(v) => self.update_altitude(Some(*v)),
                    Altitude::Invalid(_) => (), // Invalid are ignored
                },
                GeoAltitude::MergedLaser { val, .. } => self.update_altitude(Some(*val as f64)),
            }
        }
    }
}
