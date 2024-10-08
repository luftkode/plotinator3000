use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::mipmap::{MipMap2D, MipMapStrategy};

use super::util;

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct PlotData {
    plots: Vec<PlotValues>,
    plot_labels: Vec<StoredPlotLabels>,
}

impl PlotData {
    pub fn plots(&self) -> &[PlotValues] {
        &self.plots
    }

    pub fn plots_as_mut(&mut self) -> &mut Vec<PlotValues> {
        &mut self.plots
    }

    pub fn plot_labels(&self) -> &[StoredPlotLabels] {
        &self.plot_labels
    }

    pub fn plot_labels_as_mut(&mut self) -> &mut Vec<StoredPlotLabels> {
        &mut self.plot_labels
    }

    pub fn add_plot_labels(&mut self, plot_labels: StoredPlotLabels) {
        self.plot_labels.push(plot_labels);
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct PlotValues {
    raw_plot: Vec<[f64; 2]>,
    mipmap_max: MipMap2D<f64>,
    mipmap_min: MipMap2D<f64>,
    name: String,
    log_id: usize,
    // Label = "<name> #<log_id>"
    label: String,
}

type PointList<'pl> = &'pl [[f64; 2]];

impl PlotValues {
    pub fn new(raw_plot: Vec<[f64; 2]>, name: String, log_id: usize) -> Self {
        let label = format!("{name} #{log_id}");
        Self {
            mipmap_max: MipMap2D::new(raw_plot.clone(), MipMapStrategy::Max),
            mipmap_min: MipMap2D::new(raw_plot.clone(), MipMapStrategy::Min),
            raw_plot,
            name,
            log_id,
            label,
        }
    }

    pub fn get_raw(&self) -> PointList {
        &self.raw_plot
    }

    pub fn get_level(&self, level: usize) -> Option<(PointList, PointList)> {
        let mipmap_min = self.mipmap_min.get_level(level)?;
        let mipmap_max = self.mipmap_max.get_level(level)?;
        Some((mipmap_min, mipmap_max))
    }

    pub fn get_level_or_max(&self, level: usize) -> (PointList, PointList) {
        (
            self.mipmap_min.get_level_or_max(level),
            self.mipmap_max.get_level_or_max(level),
        )
    }

    pub fn get_max_level(&self) -> (PointList, PointList) {
        (
            self.mipmap_min.get_max_level(),
            self.mipmap_max.get_max_level(),
        )
    }

    pub fn mipmap_levels(&self) -> usize {
        self.mipmap_min.num_levels()
    }

    pub fn get_scaled_mipmap_levels(
        &mut self,
        pixel_width: usize,
        x_bounds: (usize, usize),
    ) -> usize {
        self.mipmap_min.get_level_match(pixel_width, x_bounds)
    }

    /// Apply an offset to the plot based on the difference to the supplied [`DateTime<Utc>`]
    pub fn offset_plot(&mut self, new_start_date: DateTime<Utc>) {
        util::offset_data_iter(self.raw_plot.iter_mut(), new_start_date);
        self.recalc_mipmaps();
    }

    fn recalc_mipmaps(&mut self) {
        self.mipmap_min = MipMap2D::new(self.raw_plot.clone(), MipMapStrategy::Min);
        self.mipmap_max = MipMap2D::new(self.raw_plot.clone(), MipMapStrategy::Max);
    }

    /// Returns a borrowed list of all plot points
    pub fn raw_plot(&self) -> PointList {
        &self.raw_plot
    }

    /// Name of Plot, e.g. `RPM` or `Pid err`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The ID of the log that the plot belongs to
    pub fn log_id(&self) -> usize {
        self.log_id
    }

    /// Label of the plot which includes the log id ie. `"<name> #<log_id"`
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Represents all the plotlabels from a given log
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct StoredPlotLabels {
    pub log_id: usize,
    pub label_points: Vec<PlotLabel>,
}

impl StoredPlotLabels {
    pub fn new(label_points: Vec<([f64; 2], String)>, log_id: usize) -> Self {
        Self {
            label_points: label_points.into_iter().map(PlotLabel::from).collect(),
            log_id,
        }
    }

    pub fn labels(&self) -> &[PlotLabel] {
        &self.label_points
    }

    /// Apply an offset to the plot labels based on the difference to the supplied [`DateTime<Utc>`]
    pub fn offset_labels(&mut self, new_start_date: DateTime<Utc>) {
        util::offset_data_iter(self.label_points_mut(), new_start_date);
    }

    // Returns mutable references to the points directly
    fn label_points_mut(&mut self) -> impl Iterator<Item = &mut [f64; 2]> {
        self.label_points.iter_mut().map(|label| &mut label.point)
    }

    pub fn log_id(&self) -> usize {
        self.log_id
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct PlotLabel {
    pub point: [f64; 2],
    pub text: String,
}

impl PlotLabel {
    pub fn new(point: [f64; 2], text: String) -> Self {
        Self { point, text }
    }

    pub fn point(&self) -> [f64; 2] {
        self.point
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl From<([f64; 2], String)> for PlotLabel {
    fn from(value: ([f64; 2], String)) -> Self {
        Self {
            point: value.0,
            text: value.1,
        }
    }
}
