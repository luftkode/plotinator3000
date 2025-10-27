use egui::Color32;
use plotinator_ui_util::ExpectedPlotRange;
use serde::{Deserialize, Serialize};

use crate::prelude::DataType;

/// [`RawPlot`] represents some plottable data from a log, e.g. RPM measurements
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RawPlotCommon {
    legend_name: String,
    points: Vec<[f64; 2]>,
    ty: DataType,
    color: Option<Color32>,
}

impl RawPlotCommon {
    /// Instantiate a new [`RawPlotCommon`] with automatic color assignment
    ///
    /// See [`RawPlotBuilder`] for how to easily construct them and turn them into [`RawPlot`]
    pub fn new(dataset_name: impl AsRef<str>, points: Vec<[f64; 2]>, ty: DataType) -> Self {
        Self {
            legend_name: ty.legend_name(dataset_name.as_ref()),
            points,
            color: None,
            ty,
        }
    }

    /// Instantiate a new [`RawPlotCommon`] with manual color assignment
    ///
    /// See [`RawPlotBuilder`] for how to easily construct them and turn them into [`RawPlot`]
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
