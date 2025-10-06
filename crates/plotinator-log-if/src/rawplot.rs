use egui::Color32;
use serde::{Deserialize, Serialize};

use crate::{
    prelude::ExpectedPlotRange,
    rawplot::path_data::{AuxiliaryGeoSpatialData, GeoSpatialData, GeoSpatialDataBuildOutput},
};

pub mod path_data;

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub enum RawPlot {
    Generic {
        common: RawPlotCommon,
    },
    /// Data with at least time and coordinates lat/lon, might also include heading and altitude
    GeoSpatial {
        geo_data: GeoSpatialData,
    },
    /// Data with at least time and either heading, altitude, or velocity
    AuxGeoSpatial {
        aux_data: AuxiliaryGeoSpatialData,
    },
    /// Flags that can either be 0 or 1
    Boolean {
        common: RawPlotCommon,
    },
}

impl From<RawPlotCommon> for RawPlot {
    fn from(common: RawPlotCommon) -> Self {
        Self::Generic { common }
    }
}

impl From<GeoSpatialData> for RawPlot {
    fn from(geo_data: GeoSpatialData) -> Self {
        Self::GeoSpatial { geo_data }
    }
}

impl From<AuxiliaryGeoSpatialData> for RawPlot {
    fn from(aux_data: AuxiliaryGeoSpatialData) -> Self {
        Self::AuxGeoSpatial { aux_data }
    }
}

impl From<GeoSpatialDataBuildOutput> for RawPlot {
    fn from(geo_data_build_output: GeoSpatialDataBuildOutput) -> Self {
        match geo_data_build_output {
            GeoSpatialDataBuildOutput::GeoSpatialData(geo_spatial_data) => geo_spatial_data.into(),
            GeoSpatialDataBuildOutput::AuxGeoSpatialData(aux_geo_data) => aux_geo_data.into(),
        }
    }
}

/// [`RawPlot`] represents some plottable data from a log, e.g. RPM measurements
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RawPlotCommon {
    name: String,
    points: Vec<[f64; 2]>,
    expected_range: ExpectedPlotRange,
    color: Option<Color32>,
}

impl RawPlotCommon {
    pub fn new(
        name: impl Into<String>,
        points: Vec<[f64; 2]>,
        expected_range: ExpectedPlotRange,
    ) -> Self {
        Self {
            name: name.into(),
            points,
            expected_range,
            color: None,
        }
    }

    pub fn with_color(
        name: String,
        points: Vec<[f64; 2]>,
        expected_range: ExpectedPlotRange,
        color: Color32,
    ) -> Self {
        Self {
            name,
            points,
            expected_range,
            color: Some(color),
        }
    }

    pub fn color(&self) -> Option<Color32> {
        self.color
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn points(&self) -> &[[f64; 2]] {
        &self.points
    }
    pub fn points_as_mut(&mut self) -> &mut [[f64; 2]] {
        &mut self.points
    }
    pub fn expected_range(&self) -> ExpectedPlotRange {
        self.expected_range
    }
    /// Get the label of the plot from the given `id` ie. `"<name> #<id>"`
    pub fn label_from_id(&self, id: u16) -> String {
        format!("{} #{id}", self.name())
    }
}
