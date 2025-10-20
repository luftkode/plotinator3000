use std::borrow::Cow;

use egui::Color32;
use plotinator_ui_util::ExpectedPlotRange;
use serde::{Deserialize, Serialize};

use crate::rawplot::path_data::{
    AuxiliaryGeoSpatialData, GeoSpatialDataset, PrimaryGeoSpatialData,
};

pub mod path_data;

/// Helper builder to build generic [`RawPlot`] with less boilerplate
pub struct RawPlotBuilder {
    dataset_name: String,
    raw_plots: Vec<RawPlotCommon>,
}

impl RawPlotBuilder {
    pub fn new(dataset_name: impl Into<String>) -> Self {
        Self {
            dataset_name: dataset_name.into(),
            raw_plots: vec![],
        }
    }

    pub fn add(mut self, points: Vec<[f64; 2]>, ty: DataType) -> Self {
        self.raw_plots
            .push(RawPlotCommon::new(self.dataset_name.clone(), points, ty));
        self
    }

    pub fn build(mut self) -> Vec<RawPlot> {
        self.raw_plots.retain(|rp| {
            let points = rp.points().len();
            if points > 2 {
                true
            } else {
                log::warn!(
                    "Removing {}, points={points} but the minimum for plotting is 2",
                    rp.legend_name()
                );
                false
            }
        });
        self.raw_plots.into_iter().map(Into::into).collect()
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub enum RawPlot {
    Generic {
        common: RawPlotCommon,
    },
    /// Either Primary geo spatial data with at least coordinates lat/lon, with optional heading and altitude or
    /// auxiliary geo spatial data with one or more of: Altitude, velocity, and heading
    GeoSpatialDataset(GeoSpatialDataset),
}

impl From<RawPlotCommon> for RawPlot {
    fn from(common: RawPlotCommon) -> Self {
        Self::Generic { common }
    }
}

impl From<PrimaryGeoSpatialData> for RawPlot {
    fn from(geo_data: PrimaryGeoSpatialData) -> Self {
        Self::GeoSpatialDataset(GeoSpatialDataset::PrimaryGeoSpatialData(geo_data))
    }
}

impl From<AuxiliaryGeoSpatialData> for RawPlot {
    fn from(aux_data: AuxiliaryGeoSpatialData) -> Self {
        Self::GeoSpatialDataset(GeoSpatialDataset::AuxGeoSpatialData(aux_data))
    }
}

impl From<GeoSpatialDataset> for RawPlot {
    fn from(geo_data: GeoSpatialDataset) -> Self {
        Self::GeoSpatialDataset(geo_data)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum DataType {
    /// Altitude from a laser range finder in Meters.
    AltitudeLaser,
    /// Altitude above Mean Sea Level from a GNSS receiver
    AltitudeMSL,
    /// Ellipsoidal height (aka. geodetic height) from a GNSS receiver
    AltitudeEllipsoidal,
    /// km/h
    Velocity,
    /// Pitch°
    Pitch,
    /// Roll°
    Roll,
    /// Yaw°
    Yaw,
    /// UTM east in meters
    UtmEasting,
    /// UTM north in meters
    UtmNorthing,
    /// Latitude°
    Latitude,
    /// Longitude°
    Longitude,
    /// Heading° aka. course over ground
    Heading,
    /// Power/wattage [W]
    Power { name: String },
    /// Electrical Current [A] with `suffix` e.g. if suffix=bifrost you get a legend name of `Current bifrost [I] (TX Bifrost)`
    Current { suffix: Option<String> },
    /// Voltage [V]
    Voltage { name: String },
    /// Resistance [Ω]
    ElectricalResistance { name: String },
    /// In nano Tesla
    MagneticFlux,
    /// Temperature in celsius °C
    Temperature { name: String },
    /// Flag, something that is 0 or 1, with the `name` in the string
    Bool { name: String, default_hidden: bool },
    /// Time/duration e.g. engine runtime in hours [h]
    Time { name: String, unit: String },
    /// Time delta, e.g. the difference between system time and GPS time or sample time
    TimeDelta { name: String, unit: String },
    /// Such as PWM
    Percentage { name: String },
    /// Other [`DataType`], name and optionally `unit` e.g. would be `Heading°` if it was specified for heading, or `RPM` for RPM.
    Other {
        name: String,
        unit: Option<String>,
        plot_range: ExpectedPlotRange,
        default_hidden: bool,
    },
}

impl<'s> DataType {
    pub fn name(&'s self) -> Cow<'s, str> {
        match self {
            Self::AltitudeLaser => "Altitude [m]".into(),
            Self::AltitudeMSL => "Altitude [MSL, m]".into(),
            Self::AltitudeEllipsoidal => "Altitude [Geo, m]".into(),
            Self::Velocity => "Velocity [km/h]".into(),
            Self::Pitch => "Pitch°".into(),
            Self::Roll => "Roll°".into(),
            Self::Yaw => "Yaw°".into(),
            Self::Latitude => "Latitude°".into(),
            Self::Longitude => "Longitude°".into(),
            Self::UtmEasting => "East [m]".into(),
            Self::UtmNorthing => "North [m]".into(),
            Self::Heading => "Heading°".into(),
            Self::Current { suffix } => format!(
                "Current{suf} [A]",
                suf = suffix.as_ref().map(|s| format!(" {s}")).unwrap_or_default()
            )
            .into(),
            Self::Power { name } => format!("{name} [W]").into(),
            Self::Voltage { name } => format!("{name} [V]").into(),
            Self::ElectricalResistance { name } => format!("{name} [Ω]").into(),
            Self::MagneticFlux => "Flux [nT]".into(),
            Self::Temperature { name } => format!("{name} °C").into(),
            Self::Bool { name, .. } => format!("{name} [bool]").into(),
            Self::Time { name, unit } => format!("{name} [{unit}]").into(),
            Self::TimeDelta { name, unit } => format!("{name} Δ [{unit}]").into(),
            Self::Percentage { name } => format!("{name} [%]").into(),
            Self::Other {
                name,
                unit,
                plot_range: _,
                default_hidden: _,
            } => format!(
                "{name}{unit_str}",
                unit_str = unit.as_ref().map(|u| format!(" [{u}]")).unwrap_or_default()
            )
            .into(),
        }
    }

    /// Returns the legend name for the [`DataType`] when it comes from a loaded file
    ///
    /// e.g. `Velocity [km/h] (frame-gps)`
    pub fn legend_name(&self, dataset_name: &str) -> String {
        let mut legend = self.name().into_owned();
        if !dataset_name.is_empty() {
            legend.push(' ');
            legend.push('(');
            legend.push_str(dataset_name);
            legend.push(')');
        }
        legend
    }

    /// Returns the legend name for the [`DataType`] when it comes via MQTT
    ///
    /// e.g. `/dt/tc/frame-gps/1 Velocity [km/h]`
    pub fn legend_name_mqtt(&self, topic: &str) -> String {
        let mut legend = topic.to_owned();
        legend.push(' ');
        legend.push_str(&self.name());
        legend
    }

    /// Bool/flag with the `name` such as e.g. `UTC enabled` for a GNSS receiver
    pub fn bool(name: impl Into<String>, default_hidden: bool) -> Self {
        Self::Bool {
            name: name.into(),
            default_hidden,
        }
    }

    /// `other` data type with no unit, e.g. PDOP
    pub fn other_unitless(
        name: impl Into<String>,
        plot_range: ExpectedPlotRange,
        default_hidden: bool,
    ) -> Self {
        Self::Other {
            name: name.into(),
            unit: None,
            plot_range,
            default_hidden,
        }
    }

    /// `other` data type with degrees as unit (°)
    pub fn other_degrees(name: impl Into<String>, default_hidden: bool) -> Self {
        Self::Other {
            name: format!("{}°", name.into()),
            unit: None,
            plot_range: ExpectedPlotRange::Hundreds,
            default_hidden,
        }
    }

    /// `other` data type with velocity km/h as unit, such as velocity north, or speed accuracy
    pub fn other_velocity(name: impl Into<String>, default_hidden: bool) -> Self {
        Self::Other {
            name: name.into(),
            unit: Some("km/h".into()),
            plot_range: ExpectedPlotRange::Hundreds,
            default_hidden,
        }
    }

    /// `other` data type with distance meters as unit, such as vertical accuracy
    pub fn other_distance(name: impl Into<String>, default_hidden: bool) -> Self {
        Self::Other {
            name: name.into(),
            unit: Some("m".into()),
            plot_range: ExpectedPlotRange::Hundreds,
            default_hidden,
        }
    }

    pub fn plot_range(&self) -> ExpectedPlotRange {
        match self {
            Self::AltitudeLaser
            | Self::AltitudeMSL
            | Self::AltitudeEllipsoidal
            | Self::Velocity
            | Self::Pitch
            | Self::Roll
            | Self::Yaw
            | Self::UtmEasting
            | Self::UtmNorthing
            | Self::Latitude
            | Self::Longitude
            | Self::Heading
            | Self::Time { .. }
            | Self::Voltage { .. }
            | Self::Current { .. }
            | Self::ElectricalResistance { .. }
            | Self::Temperature { .. } => ExpectedPlotRange::Hundreds,
            Self::MagneticFlux | Self::TimeDelta { .. } | Self::Power { .. } => {
                ExpectedPlotRange::Thousands
            }
            Self::Percentage { .. } | Self::Bool { .. } => ExpectedPlotRange::Percentage,
            Self::Other { plot_range, .. } => *plot_range,
        }
    }

    pub fn default_hidden(&self) -> bool {
        match self {
            Self::AltitudeLaser
            | Self::AltitudeMSL
            | Self::AltitudeEllipsoidal
            | Self::Velocity
            | Self::Pitch
            | Self::Roll
            | Self::Yaw
            | Self::Power { .. }
            | Self::Current { .. }
            | Self::Voltage { .. }
            | Self::ElectricalResistance { .. }
            | Self::MagneticFlux
            | Self::Temperature { .. }
            | Self::Time { .. }
            | Self::TimeDelta { .. }
            | Self::Percentage { .. } => false,
            Self::Bool { default_hidden, .. } | Self::Other { default_hidden, .. } => {
                *default_hidden
            }
            Self::UtmNorthing
            | Self::UtmEasting
            | Self::Latitude
            | Self::Longitude
            | Self::Heading => true,
        }
    }
}

/// [`RawPlot`] represents some plottable data from a log, e.g. RPM measurements
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RawPlotCommon {
    legend_name: String,
    points: Vec<[f64; 2]>,
    ty: DataType,
    color: Option<Color32>,
}

impl RawPlotCommon {
    pub fn new(dataset_name: impl AsRef<str>, points: Vec<[f64; 2]>, ty: DataType) -> Self {
        Self {
            legend_name: ty.legend_name(dataset_name.as_ref()),
            points,
            color: None,
            ty,
        }
    }

    pub fn with_color(
        dataset_name: impl AsRef<str>,
        points: Vec<[f64; 2]>,
        ty: DataType,
        color: Color32,
    ) -> Self {
        Self {
            legend_name: ty.legend_name(dataset_name.as_ref()),
            points,
            color: Some(color),
            ty,
        }
    }

    pub fn ty(&self) -> &DataType {
        &self.ty
    }

    pub fn color(&self) -> Option<Color32> {
        self.color
    }
    pub fn legend_name(&self) -> &str {
        &self.legend_name
    }
    pub fn points(&self) -> &[[f64; 2]] {
        &self.points
    }
    pub fn points_as_mut(&mut self) -> &mut [[f64; 2]] {
        &mut self.points
    }
    pub fn expected_range(&self) -> ExpectedPlotRange {
        self.ty.plot_range()
    }
    pub fn default_hidden(&self) -> bool {
        self.ty.default_hidden()
    }
    /// Get the label of the plot from the given `id` ie. `"<name> #<id>"`
    pub fn label_from_id(&self, id: u16) -> String {
        format!("{} #{id}", self.legend_name())
    }
}
