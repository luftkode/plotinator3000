use egui::{Color32, Pos2, Stroke, epaint::PathShape};
use plotinator_ui_util::auto_color;
use serde::{Deserialize, Serialize};
use walkers::{Position, lat_lon};

use crate::{prelude::ExpectedPlotRange, rawplot::RawPlotCommon};

/// A single point in space and time
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub timestamp: f64,
    /// Lat/lon
    pub position: Position,
    /// Heading in degrees (0 = North, 90 = East, etc.)
    pub heading: Option<f64>,
    /// Meters
    pub altitude: Option<f64>,
    /// km/h
    pub speed: Option<f64>,
}

impl GeoPoint {
    pub fn new(timestamp: f64, (lat, lon): (f64, f64)) -> Self {
        Self {
            timestamp,
            position: lat_lon(lat, lon),
            heading: None,
            altitude: None,
            speed: None,
        }
    }

    pub fn with_heading(mut self, heading: f64) -> Self {
        self.heading = Some(heading);
        self
    }

    pub fn with_altitude(mut self, altitude: f64) -> Self {
        self.altitude = Some(altitude);
        self
    }

    pub fn with_speed(mut self, speed: f64) -> Self {
        self.speed = Some(speed);
        self
    }
}

#[derive(Default)]
pub struct GeoSpatialDataBuilder<'a, 'b, 'c, 'd, 'e, 'f> {
    name: String,
    timestamp: Option<&'a [f64]>,
    lat: Option<&'b [f64]>,
    lon: Option<&'c [f64]>,
    heading: Option<&'d [f64]>,
    altitude: Option<&'e [f64]>,
    speed: Option<&'f [f64]>,
}

impl<'a, 'b, 'c, 'd, 'e, 'f> GeoSpatialDataBuilder<'a, 'b, 'c, 'd, 'e, 'f> {
    /// Start building, supplying a name such as `GP1` or `Njord Altimeter`
    pub fn new(name: String) -> Self {
        Self {
            name,
            timestamp: None,
            lat: None,
            lon: None,
            heading: None,
            altitude: None,
            speed: None,
        }
    }
    pub fn timestamp(mut self, t: &'a [f64]) -> Self {
        self.timestamp = Some(t);
        self
    }

    pub fn lat(mut self, lat: &'b [f64]) -> Self {
        self.lat = Some(lat);
        self
    }

    pub fn lon(mut self, lon: &'c [f64]) -> Self {
        self.lon = Some(lon);
        self
    }

    pub fn heading(mut self, heading: &'d [f64]) -> Self {
        self.heading = Some(heading);
        self
    }

    pub fn altitude(mut self, altitude: &'e [f64]) -> Self {
        self.altitude = Some(altitude);
        self
    }

    pub fn speed(mut self, speed: &'f [f64]) -> Self {
        self.speed = Some(speed);
        self
    }

    pub fn build(self) -> anyhow::Result<GeoSpatialData> {
        let Self {
            name,
            timestamp,
            lat,
            lon,
            heading,
            altitude,
            speed,
        } = self;

        let ts = timestamp.ok_or_else(|| anyhow::anyhow!("timestamp data is required"))?;
        let lat = lat.ok_or_else(|| anyhow::anyhow!("lat data is required"))?;
        let lon = lon.ok_or_else(|| anyhow::anyhow!("lon data is required"))?;

        let len = ts.len().min(lat.len()).min(lon.len());
        let mut points = Vec::with_capacity(len);

        for i in 0..len {
            let mut point = GeoPoint::new(ts[i], (lat[i], lon[i]));

            if let Some(h) = heading {
                if i < h.len() {
                    point = point.with_heading(h[i]);
                }
            }
            if let Some(a) = altitude {
                if i < a.len() {
                    point = point.with_altitude(a[i]);
                }
            }
            if let Some(s) = speed {
                if i < s.len() {
                    point = point.with_speed(s[i]);
                }
            }

            points.push(point);
        }

        Ok(GeoSpatialData::new(name, points))
    }
}

/// Represents a path through space and time
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoSpatialData {
    pub name: String,
    pub points: Vec<GeoPoint>,
    pub color: Color32,

    cached: CachedValues,
}

impl GeoSpatialData {
    pub fn new(name: String, points: Vec<GeoPoint>) -> Self {
        let color = auto_color();
        let cached = CachedValues::compute(&points);
        Self {
            name,
            points,
            color,
            cached,
        }
    }

    /// Get the latitude bounds (min, max) if available
    pub fn lat_bounds(&self) -> (f64, f64) {
        self.cached.lat_min_max
    }

    /// Get the longitude bounds (min, max) if available
    pub fn lon_bounds(&self) -> (f64, f64) {
        self.cached.lon_min_max
    }

    /// Builds and returns all the [RawPlotCommon] that can be extracted from the underlying data
    pub fn raw_plots_common(&self) -> Vec<RawPlotCommon> {
        let data_len = self.points.len();
        let altitude_len = if self.points.first().is_some_and(|p| p.altitude.is_some()) {
            data_len
        } else {
            0
        };
        let heading_len = if self.points.first().is_some_and(|p| p.heading.is_some()) {
            data_len
        } else {
            0
        };
        let speed_len = if self.points.first().is_some_and(|p| p.speed.is_some()) {
            data_len
        } else {
            0
        };
        let mut latitude = Vec::with_capacity(data_len);
        let mut longitude = Vec::with_capacity(data_len);
        let mut altitude = Vec::with_capacity(altitude_len);
        let mut heading = Vec::with_capacity(heading_len);
        let mut speed = Vec::with_capacity(speed_len);
        for p in &self.points {
            let t = p.timestamp;
            latitude.push([t, p.position.y()]);
            longitude.push([t, p.position.x()]);
            if let Some(alt) = p.altitude {
                altitude.push([t, alt]);
            }
            if let Some(head) = p.heading {
                heading.push([t, head]);
            }
            if let Some(spd) = p.speed {
                speed.push([t, spd]);
            }
        }
        let color = self.color;
        let mut plots = vec![
            RawPlotCommon::with_color(
                format!("Latitude° ({})", self.name),
                latitude,
                ExpectedPlotRange::OneToOneHundred,
                color,
            ),
            RawPlotCommon::with_color(
                format!("Longitude° ({})", self.name),
                longitude,
                ExpectedPlotRange::OneToOneHundred,
                color,
            ),
        ];
        if !altitude.is_empty() {
            plots.push(RawPlotCommon::with_color(
                format!("Altitude [m] ({})", self.name),
                altitude,
                ExpectedPlotRange::OneToOneHundred,
                color,
            ));
        }
        if !heading.is_empty() {
            plots.push(RawPlotCommon::with_color(
                format!("Heading° ({})", self.name),
                heading,
                ExpectedPlotRange::OneToOneHundred,
                color,
            ));
        }
        if !speed.is_empty() {
            plots.push(RawPlotCommon::with_color(
                format!("Speed [km/h] ({})", self.name),
                speed,
                ExpectedPlotRange::OneToOneHundred,
                color,
            ));
        }

        plots.retain(|rp| {
            if rp.points().is_empty() {
                log::debug!("{} has no data", rp.name());
                false
            } else {
                true
            }
        });
        plots
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct CachedValues {
    lat_min_max: (f64, f64),
    lon_min_max: (f64, f64),
}

impl CachedValues {
    fn compute(points: &[GeoPoint]) -> Self {
        if points.is_empty() {
            return Self::default();
        }

        // Compute lat/lon bounds
        let mut lat_min = f64::INFINITY;
        let mut lat_max = f64::NEG_INFINITY;
        let mut lon_min = f64::INFINITY;
        let mut lon_max = f64::NEG_INFINITY;

        for point in points {
            let lat = point.position.y();
            let lon = point.position.x();
            lat_min = lat_min.min(lat);
            lat_max = lat_max.max(lat);
            lon_min = lon_min.min(lon);
            lon_max = lon_max.max(lon);
        }

        let lat_min_max = (lat_min, lat_max);
        let lon_min_max = (lon_min, lon_max);

        Self {
            lat_min_max,
            lon_min_max,
        }
    }

    fn create_arrow_points(
        center: Pos2,
        heading_deg: f64,
        arrow_length: f32,
        arrow_width: f32,
    ) -> Vec<Pos2> {
        let angle_rad = (90.0 - heading_deg).to_radians() as f32;

        // Calculate arrow tip position
        let tip_x = center.x + arrow_length * angle_rad.cos();
        let tip_y = center.y - arrow_length * angle_rad.sin();
        let tip = Pos2::new(tip_x, tip_y);

        // Calculate arrow base corners
        let perpendicular = angle_rad + std::f32::consts::PI / 2.0;
        let base_offset = arrow_width / 2.0;

        let left_x = center.x + base_offset * perpendicular.cos();
        let left_y = center.y - base_offset * perpendicular.sin();
        let left = Pos2::new(left_x, left_y);

        let right_x = center.x - base_offset * perpendicular.cos();
        let right_y = center.y + base_offset * perpendicular.sin();
        let right = Pos2::new(right_x, right_y);

        vec![tip, left, right]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build_geo_spatial_data() {
        let name = "Data".to_owned();
        let timestamps = &[2., 3., 4.];
        let latitude = &[5., 6., 7.];
        let longitude = &[5.5, 6.6, 7.7];
        let altitude = &[20., 30., 40.];
        let speed = &[2., 2.5, 3.];
        let heading = &[20., 19., 21.];

        let geo_data: GeoSpatialData = GeoSpatialDataBuilder::new(name)
            .timestamp(timestamps)
            .lat(latitude)
            .lon(longitude)
            .altitude(altitude)
            .speed(speed)
            .heading(heading)
            .build()
            .unwrap();
        assert_eq!(geo_data.points[0].altitude, Some(altitude[0]))
    }
}
