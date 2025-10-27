use ndarray::{ArrayBase, Ix1};
use num_traits::{AsPrimitive, PrimInt};
use serde::{Deserialize, Serialize};

use crate::{
    algorithms::{self, scale_timestamp_distances},
    prelude::DataType,
    rawplot::{
        path_data::{AuxiliaryGeoSpatialData, GeoSpatialDataset, PrimaryGeoSpatialData},
        rawplot_common::RawPlotCommon,
    },
};

pub mod data_type;
pub mod path_data;
pub mod rawplot_common;

pub trait TimeStampPrimitive: PrimInt + AsPrimitive<f64> + AsPrimitive<u64> {}
impl TimeStampPrimitive for i64 {}
impl TimeStampPrimitive for u64 {}

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

    pub fn add_timestamp_delta(mut self, timestamps: &[impl TimeStampPrimitive]) -> Self {
        self.add_timestamp_delta_rawplot(algorithms::timestamp_distances(timestamps));
        self
    }

    pub fn add_timestamp_delta_ndarray<T, S>(mut self, timestamps: &ArrayBase<S, Ix1>) -> Self
    where
        T: TimeStampPrimitive,
        S: ndarray::Data<Elem = T>,
    {
        self.add_timestamp_delta_rawplot(algorithms::timestamp_distances_ndarray(timestamps));
        self
    }

    fn add_timestamp_delta_rawplot(&mut self, mut timestamp_delta_ns: Vec<[f64; 2]>) {
        let scaled_unit = scale_timestamp_distances(&mut timestamp_delta_ns);
        self.raw_plots.push(RawPlotCommon::new(
            self.dataset_name.clone(),
            timestamp_delta_ns,
            DataType::TimeDelta {
                name: "Sample".into(),
                unit: scaled_unit,
            },
        ));
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
