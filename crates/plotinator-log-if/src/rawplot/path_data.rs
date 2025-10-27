use std::fmt;

use anyhow::bail;
use egui::Color32;
use num_traits::AsPrimitive as _;
use plotinator_ui_util::{
    ExpectedPlotRange, auto_color_plot_area, auto_terrain_safe_color, invalid_data_color,
};
use serde::{Deserialize, Serialize};
use walkers::{Position, lat_lon};

use crate::{
    algorithms,
    rawplot::{
        DataType, RawPlot, TimeStampPrimitive, path_data::caching::CachedValues,
        rawplot_common::RawPlotCommon,
    },
};

pub mod caching;

/// Altitude samples
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RawGeoAltitudes {
    /// Altitude source is a GNSS receiver
    Gnss(Box<[f64]>),
    /// Altitude source is a laser range finder
    Laser(Box<[f64]>),
}

impl RawGeoAltitudes {
    fn len(&self) -> usize {
        match self {
            Self::Gnss(a) | Self::Laser(a) => a.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Altitude {
    Valid(f64),
    Invalid(f64),
}

impl Altitude {
    #[inline(always)]
    fn new(altitude: f64, min_max: Option<(f64, f64)>) -> Self {
        if let Some((min, max)) = min_max
            && (altitude < min || altitude > max)
        {
            Self::Invalid(altitude)
        } else {
            Self::Valid(altitude)
        }
    }

    fn val(&self) -> f64 {
        match self {
            Self::Valid(v) | Self::Invalid(v) => *v,
        }
    }
}

/// Altitude sample
///
/// Laser is preferred over GNSS, so if [`AuxiliaryGeoSpatialData`] is loaded that is compatible with an existing [`GeoSpatialData`] that only has
/// altitude from `GNSS` then the altitude will be replaced with the laser altitude
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GeoAltitude {
    /// Altitude source is a GNSS receiver
    Gnss(Altitude),
    /// Altitude source is a laser range finder
    Laser(Altitude),
    /// Altitude source is a merged data point from a laser range finder
    /// the source index is the index in the vector that stores metadata about the source of the merged data
    MergedLaser { val: f32, source_index: u8 },
}

impl GeoAltitude {
    pub fn val(&self) -> Altitude {
        match self {
            Self::Gnss(val) | Self::Laser(val) => *val,
            Self::MergedLaser { val, .. } => Altitude::Valid(*val as f64),
        }
    }

    pub fn inner_raw(&self) -> f64 {
        self.val().val()
    }

    pub fn source(&self) -> &str {
        match self {
            Self::Gnss(_) => "GNSS Receiver",
            Self::Laser(_) | Self::MergedLaser { .. } => "Laser",
        }
    }

    pub fn is_laser(&self) -> bool {
        match self {
            Self::Gnss(_) => false,
            Self::Laser(_) | Self::MergedLaser { .. } => true,
        }
    }
    pub fn is_gnss(&self) -> bool {
        matches!(self, Self::Gnss(_))
    }
}

impl fmt::Display for GeoAltitude {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.0} m", self.val().val())
    }
}

/// A single point in space and time
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub timestamp: f64,
    /// Lat/lon
    pub position: Position,
    /// Heading in degrees (0 = North, 90 = East, etc.)
    pub heading: Option<f64>,
    /// Meters, can contain multiple values if any additional were merged.
    pub altitude: Vec<GeoAltitude>,
    /// km/h
    pub speed: Option<f64>,
}

impl GeoPoint {
    #[inline]
    pub fn new(timestamp: f64, (lat, lon): (f64, f64)) -> Self {
        Self {
            timestamp,
            position: lat_lon(lat, lon),
            heading: None,
            altitude: vec![],
            speed: None,
        }
    }

    /// Heading in degrees (0 = North, 90 = East, etc.)
    #[inline]
    pub fn with_heading(mut self, heading: f64) -> Self {
        self.heading = Some(heading);
        self
    }

    /// Meters
    #[inline]
    pub fn with_altitude(mut self, altitude: GeoAltitude) -> Self {
        self.altitude.push(altitude);
        self
    }

    /// km/h
    #[inline]
    pub fn with_speed(mut self, speed: f64) -> Self {
        self.speed = Some(speed);
        self
    }
}

#[derive(Default)]
pub struct GeoSpatialDataBuilder<'a, 'b, 'c, 'd, 'f, T: TimeStampPrimitive> {
    name: String,
    timestamp: Option<&'a [T]>,
    lat: Option<&'b [f64]>,
    lon: Option<&'c [f64]>,
    heading: Option<&'d [f64]>,
    altitude: Option<RawGeoAltitudes>,
    altitude_valid_range: Option<(f64, f64)>, // Min/Max
    speed: Option<&'f [f64]>,
}

impl<'a, 'b, 'c, 'd, 'f, T: TimeStampPrimitive> GeoSpatialDataBuilder<'a, 'b, 'c, 'd, 'f, T> {
    /// Start building, supplying a name such as `GP1` or `Njord Altimeter`
    ///
    /// At minimum, timestamps and either coordinates (both lat and lon) or another kind of auxiliary data such as
    /// altitude or speed is required, otherwise the builder will fail to build.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timestamp: None,
            lat: None,
            lon: None,
            heading: None,
            altitude: None,
            altitude_valid_range: None,
            speed: None,
        }
    }
    /// Unix nanoseconds
    pub fn timestamp(mut self, t: &'a [T]) -> Self {
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

    /// Heading in degrees (0 = North, 90 = East, etc.)
    pub fn heading(mut self, heading: &'d [f64]) -> Self {
        self.heading = Some(heading);
        self
    }

    /// Meters
    fn altitude(mut self, altitude: RawGeoAltitudes) -> Self {
        self.altitude = Some(altitude);
        self
    }

    /// Meters
    pub fn altitude_from_gnss(self, altitude: Vec<f64>) -> Self {
        self.altitude(RawGeoAltitudes::Gnss(altitude.into_boxed_slice()))
    }

    /// Meters
    pub fn altitude_from_laser(self, altitude: Vec<f64>) -> Self {
        self.altitude(RawGeoAltitudes::Laser(altitude.into_boxed_slice()))
    }

    pub fn altitude_valid_range(mut self, (min, max): (f64, f64)) -> Self {
        debug_assert_eq!(
            self.altitude_valid_range, None,
            "Altitude valid range assigned twice"
        );
        self.altitude_valid_range = Some((min, max));
        self
    }

    /// km/h
    pub fn speed(mut self, speed: &'f [f64]) -> Self {
        self.speed = Some(speed);
        self
    }

    /// Convenience build to turn the builder directly into the generic [`RawPlot`]
    pub fn build_into_rawplot(self) -> anyhow::Result<Option<RawPlot>> {
        self.build()
            .map(|maybe_geo| maybe_geo.map(|geo| geo.into()))
    }

    /// Attempt to turn the builder into [`GeoSpatialDataBuildOutput`]
    ///
    /// If the builder has coordinates it will produce a [`PrimaryGeoSpatialData`], if it instead has any of altitude, velocity, heading, it will
    /// produce a [`AuxiliaryGeoSpatialData`]
    pub fn build(self) -> anyhow::Result<Option<GeoSpatialDataset>> {
        let Self {
            name,
            timestamp,
            lat,
            lon,
            heading,
            altitude,
            altitude_valid_range,
            speed,
        } = self;
        log::info!("Building GeoSpatialDataset: {name}");

        let Some(ts) = timestamp else {
            bail!(
                "Invalid Geo Spatial dataset '{name}': timestamp data is required for building a Geo Spatial dataset"
            )
        };

        if ts.len() < 2 {
            log::warn!(
                "Cannot build GeoSpatialData from dataset '{name}' with length less than 2 points"
            );
            return Ok(None);
        }

        let mut delta_t_samples_ns = algorithms::timestamp_distances(ts);
        let scaled_unit = algorithms::scale_timestamp_distances(&mut delta_t_samples_ns);
        log::info!("Scaled {name} to {scaled_unit}");
        let delta_t_samples_plot = RawPlotCommon::new(
            name.clone(),
            delta_t_samples_ns,
            DataType::TimeDelta {
                name: "Sample".into(),
                unit: scaled_unit,
            },
        );

        if let Some(lat) = lat
            && let Some(lon) = lon
        {
            let len = ts.len().min(lat.len()).min(lon.len());
            let mut points = Vec::with_capacity(len);

            for i in 0..len {
                let mut point = GeoPoint::new(ts[i].as_(), (lat[i], lon[i]));

                if let Some(h) = heading
                    && i < h.len()
                {
                    point = point.with_heading(h[i]);
                }
                if let Some(alt) = &altitude
                    && i < alt.len()
                {
                    match alt {
                        RawGeoAltitudes::Gnss(alts) => {
                            point = point.with_altitude(GeoAltitude::Gnss(Altitude::new(
                                alts[i],
                                altitude_valid_range,
                            )));
                        }
                        RawGeoAltitudes::Laser(alts) => {
                            point = point.with_altitude(GeoAltitude::Laser(Altitude::new(
                                alts[i],
                                altitude_valid_range,
                            )));
                        }
                    }
                }
                if let Some(s) = speed
                    && i < s.len()
                {
                    point = point.with_speed(s[i]);
                }

                points.push(point);
            }

            Ok(Some(GeoSpatialDataset::PrimaryGeoSpatialData(
                PrimaryGeoSpatialData::new(name, points, delta_t_samples_plot),
            )))
        } else if heading.is_some() || altitude.is_some() || speed.is_some() {
            let aux_geo_data = Self::build_aux(
                name,
                ts.to_owned(),
                delta_t_samples_plot,
                heading,
                altitude,
                altitude_valid_range,
                speed,
            );
            Ok(Some(GeoSpatialDataset::AuxGeoSpatialData(aux_geo_data)))
        } else {
            bail!(
                "Cannot build Geo Spatial dataset '{name}' with neither longitude and latitude, or either of heading, speed, or altitude"
            );
        }
    }

    fn build_aux(
        name: String,
        timestamps: Vec<T>,
        delta_t_samples_plot: RawPlotCommon,
        heading: Option<&[f64]>,
        altitude: Option<RawGeoAltitudes>,
        altitude_valid_range: Option<(f64, f64)>,
        speed: Option<&[f64]>,
    ) -> AuxiliaryGeoSpatialData {
        let timestamps_len = timestamps.len();
        let mut aux_geo_data = AuxiliaryGeoSpatialData::new(
            name,
            timestamps.into_iter().map(|t| t.as_()).collect(),
            delta_t_samples_plot,
        );

        if let Some(hdg) = heading {
            debug_assert_eq!(hdg.len(), timestamps_len);
            aux_geo_data = aux_geo_data.with_heading(hdg.to_owned());
        }

        if let Some(alt) = altitude {
            debug_assert_eq!(alt.len(), timestamps_len);
            let mut processed_altitudes: Vec<GeoAltitude> = Vec::with_capacity(alt.len());
            match alt {
                RawGeoAltitudes::Gnss(alts) => {
                    for a in alts {
                        processed_altitudes
                            .push(GeoAltitude::Gnss(Altitude::new(a, altitude_valid_range)));
                    }
                }
                RawGeoAltitudes::Laser(alts) => {
                    for a in alts {
                        processed_altitudes
                            .push(GeoAltitude::Laser(Altitude::new(a, altitude_valid_range)));
                    }
                }
            }
            aux_geo_data = aux_geo_data.with_altitude(processed_altitudes);
        }
        if let Some(spd) = speed {
            debug_assert_eq!(spd.len(), timestamps_len);
            aux_geo_data = aux_geo_data.with_speed(spd.to_owned());
        }
        aux_geo_data
    }
}

/// Build output of the [`GeoSpatialDataBuilder`]
///
/// The contents of the builder determines the output
#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub enum GeoSpatialDataset {
    /// Primary geo spatial data has coordinates
    PrimaryGeoSpatialData(PrimaryGeoSpatialData),
    /// Auxiliary geo spatial data has either heading, altitude, or velocity
    AuxGeoSpatialData(AuxiliaryGeoSpatialData),
}

impl GeoSpatialDataset {
    pub fn len(&self) -> usize {
        match self {
            Self::PrimaryGeoSpatialData(prim) => prim.points.len(),
            Self::AuxGeoSpatialData(aux) => aux.timestamps.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::PrimaryGeoSpatialData(prim) => prim.points.is_empty(),
            Self::AuxGeoSpatialData(aux) => aux.timestamps.is_empty(),
        }
    }

    pub fn raw_plots_common(&self) -> Vec<RawPlotCommon> {
        match self {
            Self::PrimaryGeoSpatialData(prim) => prim.raw_plots_common(),
            Self::AuxGeoSpatialData(aux) => aux.raw_plots_common(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::PrimaryGeoSpatialData(prim) => prim.name.as_str(),
            Self::AuxGeoSpatialData(aux) => aux.name.as_str(),
        }
    }

    /// Is it primary geo spatial data (has coordinates)
    pub fn is_primary(&self) -> bool {
        matches!(self, Self::PrimaryGeoSpatialData(_))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergedMetadata {
    pub name: String,
    pub color: Color32,
}

/// Represents a path through space and time
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimaryGeoSpatialData {
    pub name: String,
    /// Name of the [`AuxiliaryGeoSpatialData`] that was merged into this instance (if any)
    pub merged: Vec<MergedMetadata>,
    pub points: Vec<GeoPoint>,
    pub delta_t_samples_plot: Option<RawPlotCommon>,
    pub color: Color32,
    cached: CachedValues,
}

impl PrimaryGeoSpatialData {
    pub(crate) fn new(
        name: String,
        points: Vec<GeoPoint>,
        delta_t_samples_plot: RawPlotCommon,
    ) -> Self {
        let color = auto_terrain_safe_color();
        let cached = CachedValues::compute(&points);
        Self {
            name,
            merged: vec![],
            points,
            delta_t_samples_plot: Some(delta_t_samples_plot),
            color,
            cached,
        }
    }

    /// Get the latitude bounds (min, max)
    pub fn lat_bounds(&self) -> (f64, f64) {
        self.cached.lat_bounds()
    }

    /// Get the longitude bounds (min, max) if available
    pub fn lon_bounds(&self) -> (f64, f64) {
        self.cached.lon_bounds()
    }

    /// Get the speed bounds (min, max) if available
    pub fn speed_bounds(&self) -> (f64, f64) {
        self.cached.speed_bounds()
    }

    /// Builds and returns all the [`RawPlotCommon`] that can be extracted from the underlying data
    #[allow(clippy::too_many_lines, reason = "long but simple enough")]
    pub fn raw_plots_common(&self) -> Vec<RawPlotCommon> {
        let data_len = self.points.len();
        let altitude_len = if self.points.first().is_some_and(|p| p.altitude.is_empty()) {
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
        let mut altitude_invalid_count: u64 = 0;
        let mut altitude_invalid_counts = Vec::new();
        let mut heading = Vec::with_capacity(heading_len);
        let mut speed = Vec::with_capacity(speed_len);
        for p in &self.points {
            let t = p.timestamp;
            latitude.push([t, p.position.y()]);
            longitude.push([t, p.position.x()]);
            if let Some(alt) = p.altitude.first() {
                match alt.val() {
                    Altitude::Valid(v) => altitude.push([t, v]),
                    Altitude::Invalid(_) => {
                        altitude_invalid_count += 1;
                        altitude_invalid_counts.push([t, altitude_invalid_count as f64]);
                    }
                }
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
            RawPlotCommon::with_color(&self.name, latitude, DataType::Latitude, color),
            RawPlotCommon::with_color(&self.name, longitude, DataType::Longitude, color),
        ];
        if !altitude.is_empty() {
            plots.push(RawPlotCommon::with_color(
                &self.name,
                altitude,
                if self.points.first().is_some_and(|p| {
                    p.altitude.first().is_some_and(|a| match a {
                        GeoAltitude::Laser(_) | GeoAltitude::MergedLaser { .. } => true,
                        GeoAltitude::Gnss(_) => false,
                    })
                }) {
                    DataType::AltitudeLaser
                } else {
                    // TODO: Differentiate from ellipsoid
                    DataType::AltitudeMSL
                },
                color,
            ));
            if !altitude_invalid_counts.is_empty() {
                plots.push(RawPlotCommon::with_color(
                    &self.name,
                    altitude_invalid_counts,
                    DataType::other_unitless("Invalid Count", ExpectedPlotRange::Hundreds, false),
                    invalid_data_color(),
                ));
            }
        }
        if !heading.is_empty() {
            plots.push(RawPlotCommon::with_color(
                &self.name,
                heading,
                DataType::Heading,
                color,
            ));
        }
        if !speed.is_empty() {
            plots.push(RawPlotCommon::with_color(
                &self.name,
                speed,
                DataType::Velocity,
                color,
            ));
        }

        debug_assert!(
            self.delta_t_samples_plot.is_some(),
            "No delta t samples, was raw_plots_common() called twice?"
        );
        if let Some(p) = &self.delta_t_samples_plot {
            plots.push(p.clone());
        }

        plots.retain(|rp| {
            if rp.points().len() < 2 {
                log::debug!("{} has no data", rp.legend_name());
                false
            } else {
                true
            }
        });
        plots
    }

    /// Get the time range covered by the data
    pub fn time_range(&self) -> Option<(f64, f64)> {
        Some((
            self.points.first()?.timestamp,
            self.points.last()?.timestamp,
        ))
    }
}

/// Auxiliary time-series data that can be merged with a primary [`PrimaryGeoSpatialData`]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuxiliaryGeoSpatialData {
    pub name: String,
    pub timestamps: Vec<u64>,
    pub delta_t_samples_plot: Option<RawPlotCommon>,
    pub altitudes: Option<Vec<GeoAltitude>>,
    pub invalid_altitudes_count: Option<Vec<f64>>,
    pub speeds: Option<Vec<f64>>,
    pub headings: Option<Vec<f64>>,
    pub color: Color32,
}

impl AuxiliaryGeoSpatialData {
    pub fn new(
        name: impl Into<String>,
        timestamps: Vec<u64>,
        delta_t_samples_plot: RawPlotCommon,
    ) -> Self {
        let color = auto_color_plot_area(ExpectedPlotRange::Hundreds);
        Self {
            name: name.into(),
            timestamps,
            delta_t_samples_plot: Some(delta_t_samples_plot),
            altitudes: None,
            invalid_altitudes_count: None,
            speeds: None,
            headings: None,
            color,
        }
    }

    /// Heading in degrees (0 = North, 90 = East, etc.)
    pub fn with_heading(mut self, heading: Vec<f64>) -> Self {
        debug_assert_eq!(self.timestamps.len(), heading.len());
        self.headings = Some(heading);
        self
    }

    /// Meters
    pub fn with_altitude(mut self, altitude: Vec<GeoAltitude>) -> Self {
        debug_assert_eq!(self.timestamps.len(), altitude.len());
        self.altitudes = Some(altitude);
        self
    }

    /// km/h
    pub fn with_speed(mut self, speed: Vec<f64>) -> Self {
        debug_assert_eq!(self.timestamps.len(), speed.len());
        self.speeds = Some(speed);
        self
    }

    /// Builds and returns all the [`RawPlotCommon`] that can be extracted from the underlying data
    pub fn raw_plots_common(&self) -> Vec<RawPlotCommon> {
        let color = self.color;
        let mut plots = vec![];
        if let Some(delta_t_samples_plot) = &self.delta_t_samples_plot {
            plots.push(delta_t_samples_plot.clone());
        }
        if let Some(headings) = &self.headings {
            let mut heading = Vec::with_capacity(headings.len());
            for (t, hdg) in self.timestamps.iter().zip(headings) {
                heading.push([t.as_(), *hdg]);
            }
            if heading.len() > 1 {
                plots.push(RawPlotCommon::with_color(
                    &self.name,
                    heading,
                    DataType::Heading,
                    color,
                ));
            }
        }
        if let Some(speeds) = &self.speeds {
            let mut speed = Vec::with_capacity(speeds.len());
            for (t, hdg) in self.timestamps.iter().zip(speeds) {
                speed.push([t.as_(), *hdg]);
            }
            if speed.len() > 1 {
                plots.push(RawPlotCommon::with_color(
                    &self.name,
                    speed,
                    DataType::Velocity,
                    color,
                ));
            }
        }

        if let Some(altitudes) = &self.altitudes {
            let mut altitude = Vec::with_capacity(altitudes.len());
            let mut invalid_altitude_counts = Vec::new();
            let mut invalid_altitude_count: u64 = 0;
            for (t, alt) in self.timestamps.iter().zip(altitudes) {
                match alt.val() {
                    Altitude::Valid(v) => altitude.push([t.as_(), v]),
                    Altitude::Invalid(_) => {
                        invalid_altitude_count += 1;
                        invalid_altitude_counts.push([t.as_(), invalid_altitude_count as f64]);
                    }
                }
            }
            if altitude.len() > 1 {
                plots.push(RawPlotCommon::with_color(
                    &self.name,
                    altitude,
                    DataType::AltitudeLaser,
                    color,
                ));
            }
            if invalid_altitude_counts.len() > 1 {
                log::debug!(
                    "{} has {} invalid altitudes",
                    self.name,
                    invalid_altitude_counts.len()
                );
                plots.push(RawPlotCommon::with_color(
                    &self.name,
                    invalid_altitude_counts,
                    DataType::other_unitless("Invalid Count", ExpectedPlotRange::Hundreds, false),
                    invalid_data_color(),
                ));
            }
        }
        plots
    }

    /// Check if this auxiliary data is compatible with a primary dataset
    /// Returns true if start/end timestamps align within `tolerance_ns` (nanoseconds)
    pub fn is_compatible_with(&self, primary: &PrimaryGeoSpatialData, tolerance_ns: f64) -> bool {
        log::info!(
            "Checking if primary data '{}' is compatible with '{}'",
            primary.name,
            self.name
        );

        if self.timestamps.is_empty() {
            log::debug!("Not compatible: {} has no timestamps", self.name);
            return false;
        }
        if primary.points.is_empty() {
            log::debug!("Not compatible: {} has not points", primary.name);
            return false;
        }

        let Some((aux_start, aux_end)) = self.time_range() else {
            log::debug!("Not compatible: {} has not time range", self.name);
            return false;
        };

        let Some((primary_start, primary_end)) = primary.time_range() else {
            log::debug!("Not compatible: {} has not time range", primary.name);
            return false;
        };

        let delta_start = (aux_start - primary_start).abs();
        let delta_end = (aux_end - primary_end).abs();
        let start_is_compatible = delta_start <= tolerance_ns;
        let end_is_compatible = delta_end <= tolerance_ns;
        let delta_start_s = delta_start / 1e9;
        let delta_end_s = delta_end / 1e9;
        log::info!("Delta start: {delta_start_s}s, compatible: {start_is_compatible}");
        log::info!("Delta end: {delta_end_s}s, compatible: {end_is_compatible}");
        start_is_compatible && end_is_compatible
    }

    /// Get the time range covered by the data
    pub fn time_range(&self) -> Option<(f64, f64)> {
        Some((
            self.timestamps.first()?.as_(),
            self.timestamps.last()?.as_(),
        ))
    }
}

impl PrimaryGeoSpatialData {
    /// Merge auxiliary data into this primary dataset.
    ///
    /// Only merges fields that don't already exist in the primary data
    ///
    /// Uses nearest-neighbor matching (NO interpolation) to preserve data integrity
    pub fn merge_auxiliary(
        &mut self,
        aux: &AuxiliaryGeoSpatialData,
        tolerance_ns: f64,
        color: Color32,
    ) -> Result<(), MergeError> {
        let aux_name = &aux.name;
        let primary_name = self.name.clone();
        if !aux.is_compatible_with(self, tolerance_ns) {
            log::info!("{primary_name} and {aux_name} are not compatible");
            return Err(MergeError::IncompatibleTimeRange);
        }

        let aux_times_f64: Vec<f64> = aux.timestamps.iter().map(|t| t.as_()).collect();

        let mut any_merged = false;

        // Merge altitude if we don't have it or we have it and it's GNSS and the `aux` one is laser
        let aux_has_altitude = aux.altitudes.is_some();
        let aux_altitude_laser = aux
            .altitudes
            .as_ref()
            .is_some_and(|a| a.first().is_some_and(|p| p.is_laser()));
        log::debug!(
            "Aux '{aux_name}' has altitude={aux_has_altitude}, is_laser={aux_altitude_laser}"
        );
        if let Some(aux_alt) = &aux.altitudes
            && aux_altitude_laser
        {
            self.merged.push(MergedMetadata {
                name: aux_name.clone(),
                color,
            });
            let current_merge_index = self.merged.len() - 1;
            self.merge_altitude_nearest(&aux_times_f64, aux_alt, current_merge_index);
            log::info!("Merged altitude of '{aux_name}' into '{primary_name}'");
            any_merged = true;
        } else {
            log::debug!("Altitude is not mergeable");
        }

        // Check if we already have the data fields
        let has_speed = self.points.first().is_some_and(|p| p.speed.is_some());
        let has_heading = self.points.first().is_some_and(|p| p.heading.is_some());
        if let Some(aux_spd) = &aux.speeds
            && !has_speed
        {
            self.merge_field_nearest(&aux_times_f64, aux_spd, |p, v| {
                p.speed = Some(v);
            });
            log::info!("Merged speed of '{}' into '{}'", aux.name, self.name);
            any_merged = true;
        } else {
            log::debug!("Speed is not mergeable");
        }

        if let Some(aux_hdg) = &aux.headings
            && !has_heading
        {
            self.merge_field_nearest(&aux_times_f64, aux_hdg, |p, v| {
                p.heading = Some(v);
            });
            log::info!("Merged heading of '{}' into '{}'", aux.name, self.name);
            any_merged = true;
        } else {
            log::debug!("heading is not mergeable");
        }

        if any_merged {
            self.cached = CachedValues::compute(&self.points);
            log::info!("Merged '{aux_name}' into '{}'", self.name);
        } else {
            log::debug!("Nothing to merge from '{}' into '{}'", aux.name, self.name);
        }

        Ok(())
    }

    /// Helper to merge a single field using nearest-neighbor lookup
    /// Optimized for monotonically increasing timestamps - O(n+m) complexity
    fn merge_field_nearest<F>(&mut self, aux_times: &[f64], aux_values: &[f64], mut setter: F)
    where
        F: FnMut(&mut GeoPoint, f64),
    {
        if aux_times.is_empty() || aux_values.is_empty() {
            return;
        }

        let mut aux_idx = 0;
        let last_idx = aux_times.len() - 1;

        for point in &mut self.points {
            let target = point.timestamp;

            // Advance aux_idx while the next timestamp is closer to target
            while aux_idx < last_idx {
                let curr_diff = (aux_times[aux_idx] - target).abs();
                let next_diff = (aux_times[aux_idx + 1] - target).abs();

                if next_diff < curr_diff {
                    aux_idx += 1;
                } else {
                    break;
                }
            }

            setter(point, aux_values[aux_idx]);
        }
    }

    /// Helper to merge a single field using nearest-neighbor lookup
    /// Optimized for monotonically increasing timestamps - O(n+m) complexity
    ///
    /// Prefers valid altitude samples - if nearest is invalid, checks neighbors
    fn merge_altitude_nearest(
        &mut self,
        aux_times: &[f64],
        altitudes: &[GeoAltitude],
        merge_index: usize,
    ) {
        if aux_times.is_empty() {
            return;
        }

        let mut aux_idx = 0;
        let last_idx = aux_times.len() - 1;

        for point in &mut self.points {
            let target = point.timestamp;

            // Advance aux_idx while the next timestamp is closer to target
            while aux_idx < last_idx {
                let curr_diff = (aux_times[aux_idx] - target).abs();
                let next_diff = (aux_times[aux_idx + 1] - target).abs();

                if next_diff < curr_diff {
                    aux_idx += 1;
                } else {
                    break;
                }
            }

            // Check if the nearest altitude is valid
            let nearest_altitude = altitudes[aux_idx];
            let selected_altitude = match nearest_altitude.val() {
                Altitude::Valid(_) => nearest_altitude,
                Altitude::Invalid(_) => {
                    // Try to find a valid altitude in the two nearest neighbors
                    let prev_idx = aux_idx.saturating_sub(1);
                    let next_idx = (aux_idx + 1).min(last_idx);

                    let prev_valid =
                        aux_idx > 0 && matches!(altitudes[prev_idx].val(), Altitude::Valid(_));
                    let next_valid = aux_idx < last_idx
                        && matches!(altitudes[next_idx].val(), Altitude::Valid(_));

                    if prev_valid && next_valid {
                        // Both neighbors are valid, pick the closer one
                        let prev_diff = (aux_times[prev_idx] - target).abs();
                        let next_diff = (aux_times[next_idx] - target).abs();
                        if prev_diff < next_diff {
                            altitudes[prev_idx]
                        } else {
                            altitudes[next_idx]
                        }
                    } else if prev_valid {
                        altitudes[prev_idx]
                    } else if next_valid {
                        altitudes[next_idx]
                    } else {
                        // No valid neighbors, use the invalid one
                        nearest_altitude
                    }
                }
            };

            let val = selected_altitude.inner_raw() as f32;
            point.altitude.push(GeoAltitude::MergedLaser {
                val,
                source_index: merge_index as u8,
            });
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MergeError {
    IncompatibleTimeRange,
    FieldAlreadyExists,
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompatibleTimeRange => {
                write!(
                    f,
                    "Auxiliary data time range doesn't align with primary dataset"
                )
            }
            Self::FieldAlreadyExists => {
                write!(f, "Field already exists in primary dataset")
            }
        }
    }
}

impl std::error::Error for MergeError {}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use testresult::TestResult;

    #[test]
    fn test_build_geo_spatial_data() -> TestResult {
        let name = "Data".to_owned();
        let timestamps: &[i64] = &[2, 3, 4];
        let latitude = &[5., 6., 7.];
        let longitude = &[5.5, 6.6, 7.7];
        let altitude = vec![20., 30., 40.];
        let speed = &[2., 2.5, 3.];
        let heading = &[20., 19., 21.];

        let GeoSpatialDataset::PrimaryGeoSpatialData(geo_data) = GeoSpatialDataBuilder::new(name)
            .timestamp(timestamps)
            .lat(latitude)
            .lon(longitude)
            .altitude_from_gnss(altitude.clone())
            .speed(speed)
            .heading(heading)
            .build()?
            .unwrap()
        else {
            panic!();
        };
        assert_eq!(
            geo_data.points[0].altitude[0],
            GeoAltitude::Gnss(Altitude::Valid(altitude[0]))
        );
        Ok(())
    }

    #[test]
    fn test_merge_preserves_primary_frequency() -> TestResult {
        // Primary dataset at 1 Hz
        let primary_times: Vec<u64> = vec![1_000_000_000, 2_000_000_000, 3_000_000_000];
        let lat = vec![55.0, 55.1, 55.2];
        let lon = vec![12.0, 12.1, 12.2];

        let GeoSpatialDataset::PrimaryGeoSpatialData(mut primary) =
            GeoSpatialDataBuilder::new("Primary")
                .timestamp(&primary_times)
                .lat(&lat)
                .lon(&lon)
                .build()?
                .unwrap()
        else {
            panic!();
        };

        // Auxiliary dataset at ~3 Hz (higher frequency)
        let aux_times = vec![
            1_000_000_000,
            1_330_000_000,
            1_660_000_000,
            2_000_000_000,
            2_330_000_000,
            2_660_000_000,
            3_000_000_000,
        ];
        let aux_altitude = vec![
            GeoAltitude::Laser(Altitude::Valid(100.0)),
            GeoAltitude::Laser(Altitude::Valid(105.0)),
            GeoAltitude::Laser(Altitude::Valid(110.0)),
            GeoAltitude::Laser(Altitude::Valid(115.0)),
            GeoAltitude::Laser(Altitude::Valid(120.0)),
            GeoAltitude::Laser(Altitude::Valid(125.0)),
            GeoAltitude::Laser(Altitude::Valid(130.0)),
        ];

        let aux = AuxiliaryGeoSpatialData::new(
            "Altimeter",
            aux_times,
            RawPlotCommon::new("foo", vec![], DataType::UtmEasting),
        )
        .with_altitude(aux_altitude);

        primary.merge_auxiliary(&aux, 5.0e9, Color32::WHITE)?;

        // Result should still be 3 points (primary frequency preserved)
        assert_eq!(primary.points.len(), 3);

        // Check that nearest values were selected (not interpolated)
        assert_eq!(
            primary.points[0].altitude.first().map(|v| v.inner_raw()),
            Some(100.0)
        ); // Exact match
        assert_eq!(
            primary.points[1].altitude.first().map(|v| v.inner_raw()),
            Some(115.0)
        ); // Exact match
        assert_eq!(
            primary.points[2].altitude.first().map(|v| v.inner_raw()),
            Some(130.0)
        ); // Exact match
        Ok(())
    }

    #[test]
    fn test_exact_matches() -> TestResult {
        let primary_times: Vec<i64> = vec![1_000_000_000, 2_000_000_000, 3_000_000_000];
        let lat = vec![55.0, 55.1, 55.2];
        let lon = vec![12.0, 12.1, 12.2];

        let GeoSpatialDataset::PrimaryGeoSpatialData(mut primary) =
            GeoSpatialDataBuilder::new("Test")
                .timestamp(&primary_times)
                .lat(&lat)
                .lon(&lon)
                .build()?
                .unwrap()
        else {
            panic!()
        };

        let aux_times = vec![1_000_000_000, 2_000_000_000, 3_000_000_000];
        let aux_altitude = vec![
            GeoAltitude::Laser(Altitude::Valid(100.0)),
            GeoAltitude::Laser(Altitude::Valid(200.0)),
            GeoAltitude::Laser(Altitude::Valid(300.0)),
        ];

        let aux = AuxiliaryGeoSpatialData::new(
            "aux",
            aux_times,
            RawPlotCommon::new("foo", vec![], DataType::UtmEasting),
        )
        .with_altitude(aux_altitude);

        primary.merge_auxiliary(&aux, 1.0e8, Color32::WHITE)?;

        assert_eq!(
            primary.points[0].altitude.first().map(|v| v.inner_raw()),
            Some(100.0)
        );
        assert_eq!(
            primary.points[1].altitude.first().map(|v| v.inner_raw()),
            Some(200.0)
        );
        assert_eq!(
            primary.points[2].altitude.first().map(|v| v.inner_raw()),
            Some(300.0)
        );
        Ok(())
    }

    #[test]
    fn test_primary_lower_frequency() -> TestResult {
        // Primary at 1 Hz
        let primary_times: Vec<i64> = vec![1_000_000_000, 2_000_000_000, 3_000_000_000];
        let lat = vec![55.0, 55.1, 55.2];
        let lon = vec![12.0, 12.1, 12.2];

        let GeoSpatialDataset::PrimaryGeoSpatialData(mut primary) =
            GeoSpatialDataBuilder::new("Test")
                .timestamp(&primary_times)
                .lat(&lat)
                .lon(&lon)
                .build()?
                .unwrap()
        else {
            panic!()
        };

        // Aux at higher frequency
        let aux_times = vec![
            1_000_000_000,
            1_400_000_000,
            1_800_000_000,
            2_000_000_000,
            2_600_000_000,
            3_000_000_000,
        ];
        let aux_altitude = vec![
            GeoAltitude::Laser(Altitude::Valid(100.)),
            GeoAltitude::Laser(Altitude::Valid(110.)),
            GeoAltitude::Laser(Altitude::Valid(120.)),
            GeoAltitude::Laser(Altitude::Valid(200.)),
            GeoAltitude::Laser(Altitude::Valid(250.)),
            GeoAltitude::Laser(Altitude::Valid(300.)),
        ];
        let aux = AuxiliaryGeoSpatialData::new(
            "aux",
            aux_times,
            RawPlotCommon::new("foo", vec![], DataType::UtmEasting),
        )
        .with_altitude(aux_altitude);

        primary.merge_auxiliary(&aux, 1.0e8, Color32::WHITE)?;

        // Should pick nearest neighbors
        assert_eq!(
            primary.points[0].altitude.first().map(|v| v.inner_raw()),
            Some(100.0)
        ); // Exact: 1.0e9
        assert_eq!(
            primary.points[1].altitude.first().map(|v| v.inner_raw()),
            Some(200.0)
        ); // Exact: 2.0e9
        assert_eq!(
            primary.points[2].altitude.first().map(|v| v.inner_raw()),
            Some(300.0)
        ); // Exact: 3.0e9
        Ok(())
    }

    #[test]
    fn test_primary_higher_frequency() -> TestResult {
        // Primary at higher frequency - 2Hz
        let primary_times: Vec<i64> = vec![
            1_000_000_000,
            1_500_000_000,
            2_000_000_000,
            2_500_000_000,
            3_000_000_000,
        ];

        let lat = vec![55.0, 55.05, 55.1, 55.15, 55.2];
        let lon = vec![12.0, 12.05, 12.1, 12.15, 12.2];

        let GeoSpatialDataset::PrimaryGeoSpatialData(mut primary) =
            GeoSpatialDataBuilder::new("Test")
                .timestamp(&primary_times)
                .lat(&lat)
                .lon(&lon)
                .build()?
                .unwrap()
        else {
            panic!()
        };

        // Aux at 1 Hz
        let aux_times = vec![1_000_000_000, 2_000_000_000, 3_000_000_000];
        let aux_altitude = vec![
            GeoAltitude::Laser(Altitude::Valid(100.)),
            GeoAltitude::Laser(Altitude::Valid(200.)),
            GeoAltitude::Laser(Altitude::Valid(300.)),
        ];

        let aux = AuxiliaryGeoSpatialData {
            name: "Aux".to_owned(),
            timestamps: aux_times,
            delta_t_samples_plot: None,
            altitudes: Some(aux_altitude),
            invalid_altitudes_count: None,
            speeds: None,
            headings: None,
            color: Color32::RED,
        };

        primary
            .merge_auxiliary(&aux, 1.0e8, Color32::WHITE)
            .unwrap();

        assert_eq!(
            // 1.0 closest to 1.0
            primary.points[0].altitude.first().copied(),
            Some(GeoAltitude::MergedLaser {
                val: 100.,
                source_index: 0
            })
        );
        assert_eq!(
            // 1.5 closest to 1.0
            primary.points[1].altitude.first().copied(),
            Some(GeoAltitude::MergedLaser {
                val: 100.,
                source_index: 0
            })
        );
        assert_eq!(
            // 2.0 closest to 2.0
            primary.points[2].altitude.first().copied(),
            Some(GeoAltitude::MergedLaser {
                val: 200.,
                source_index: 0
            })
        );
        assert_eq!(
            // 2.5 closest to 2.0
            primary.points[3].altitude.first().copied(),
            Some(GeoAltitude::MergedLaser {
                val: 200.,
                source_index: 0
            })
        );
        assert_eq!(
            // 3.0 closest to 3.0
            primary.points[4].altitude.first().copied(),
            Some(GeoAltitude::MergedLaser {
                val: 300.,
                source_index: 0
            })
        );
        Ok(())
    }

    #[test]
    fn test_boundary_conditions() -> TestResult {
        let primary_times: Vec<i64> = vec![500_000_000, 1_500_000_000, 3_500_000_000];
        let lat = vec![55.0, 55.1, 55.2];
        let lon = vec![12.0, 12.1, 12.2];

        let GeoSpatialDataset::PrimaryGeoSpatialData(mut primary) =
            GeoSpatialDataBuilder::new("Test")
                .timestamp(&primary_times)
                .lat(&lat)
                .lon(&lon)
                .build()?
                .unwrap()
        else {
            panic!()
        };

        let aux_times = vec![1_000_000_000, 2_000_000_000, 3_000_000_000];
        let aux_altitude_laser1 = vec![
            GeoAltitude::Laser(Altitude::Valid(100.)),
            GeoAltitude::Laser(Altitude::Valid(200.)),
            GeoAltitude::Laser(Altitude::Valid(300.)),
        ];
        let aux_altitude_laser2 = vec![
            GeoAltitude::Laser(Altitude::Valid(101.)),
            GeoAltitude::Laser(Altitude::Valid(201.)),
            GeoAltitude::Laser(Altitude::Valid(301.)),
        ];

        let aux_laser1 = AuxiliaryGeoSpatialData::new(
            "Aux-laser-1",
            aux_times.clone(),
            RawPlotCommon::new("foo", vec![], DataType::UtmEasting),
        )
        .with_altitude(aux_altitude_laser1);
        primary.merge_auxiliary(&aux_laser1, 1.0e9, Color32::WHITE)?;

        // Before range: picks first
        assert_eq!(
            primary.points[0].altitude.first().copied(),
            Some(GeoAltitude::MergedLaser {
                val: 100.,
                source_index: 0
            })
        );
        // Middle: picks closest
        assert_eq!(
            primary.points[1].altitude.first().copied(),
            Some(GeoAltitude::MergedLaser {
                val: 100.,
                source_index: 0
            })
        ); // 1.5 closer to 1.0 than 2.0
        // After range: picks last
        assert_eq!(
            primary.points[2].altitude.first().copied(),
            Some(GeoAltitude::MergedLaser {
                val: 300.,
                source_index: 0
            })
        );

        let aux_laser2 = AuxiliaryGeoSpatialData::new(
            "Aux-laser-2",
            aux_times,
            RawPlotCommon::new("foo", vec![], DataType::UtmEasting),
        )
        .with_altitude(aux_altitude_laser2);

        assert!(aux_laser2.is_compatible_with(&primary, 1.0e9));

        primary.merge_auxiliary(&aux_laser2, 1.0e9, Color32::WHITE)?;

        // Before range: picks first
        assert_eq!(
            primary.points[0].altitude.get(1).copied(),
            Some(GeoAltitude::MergedLaser {
                val: 101.,
                source_index: 1
            })
        );
        // Middle: picks closest
        assert_eq!(
            primary.points[1].altitude.get(1).copied(),
            Some(GeoAltitude::MergedLaser {
                val: 101.,
                source_index: 1
            })
        ); // 1.5 closer to 1.0 than 2.0
        // After range: picks last
        assert_eq!(
            primary.points[2].altitude.get(1).copied(),
            Some(GeoAltitude::MergedLaser {
                val: 301.,
                source_index: 1
            })
        );

        Ok(())
    }
}
