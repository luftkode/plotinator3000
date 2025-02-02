use chrono::{DateTime, Utc};
use egui::Color32;
use log_if::prelude::RawPlot;
use serde::{Deserialize, Serialize};

use crate::mipmap::{MipMap2D, MipMapStrategy};

use super::util;

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct PlotData {
    plots: Vec<PlotValues>,
    plot_labels: Vec<StoredPlotLabels>,
    next_auto_color_idx: usize,
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

    /// Returns whether [`PlotData`] contains a plot with the label `plot_label`.
    ///
    /// The label is on the form `"<name> #<log_id>"`
    pub fn contains_plot(&self, plot_label: &str) -> bool {
        self.plots.iter().any(|p| p.label() == plot_label)
    }

    /// Adds a plot to the [`PlotData`] collection if another plot with the same label doesn't already exist
    pub fn add_plot_if_not_exists(&mut self, raw_plot: &RawPlot, log_id: u16) {
        let mut plot_label = String::with_capacity(30); // Approx. enough to not reallocate
        plot_label.push('#');
        plot_label.push_str(&log_id.to_string());
        plot_label.push(' ');
        plot_label.push_str(raw_plot.name());
        if !self.contains_plot(&plot_label) {
            let new_plot = PlotValues::new(
                raw_plot.points().to_vec(),
                raw_plot.name().to_owned(),
                log_id,
            )
            .color(self.auto_color());
            self.plots.push(new_plot);
        }
    }

    fn auto_color(&mut self) -> Color32 {
        // source: https://docs.rs/egui_plot/0.29.0/src/egui_plot/plot_ui.rs.html#21
        // should be replaced/updated if they improve their implementation or provide a public API for this
        let i = self.next_auto_color_idx;
        self.next_auto_color_idx += 1;
        let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
        let h = i as f32 * golden_ratio;
        egui::epaint::Hsva::new(h, 0.85, 0.5, 1.0).into() // TODO(emilk): OkLab or some other perspective color space
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct PlotValues {
    raw_plot: Vec<[f64; 2]>,
    mipmap_max: MipMap2D<f64>,
    mipmap_min: MipMap2D<f64>,
    name: String,
    log_id: u16,
    // Label = "<name> #<log_id>"
    label: String,
    color: Color32,
    highlight: bool,
}

type PointList<'pl> = &'pl [[f64; 2]];

impl PlotValues {
    // Don't mipmap/downsample to more than this amount of elements
    const MIPMAP_MIN_ELEMENTS: usize = 512;

    pub fn new(raw_plot: Vec<[f64; 2]>, name: String, log_id: u16) -> Self {
        let label = format!("{name} #{log_id}");
        Self {
            mipmap_max: MipMap2D::without_base(
                &raw_plot,
                MipMapStrategy::Max,
                Self::MIPMAP_MIN_ELEMENTS,
            ),
            mipmap_min: MipMap2D::without_base(
                &raw_plot,
                MipMapStrategy::Min,
                Self::MIPMAP_MIN_ELEMENTS,
            ),
            raw_plot,
            name,
            log_id,
            label,
            // Color32::TRANSPARENT means we auto assign one
            color: Color32::TRANSPARENT,
            highlight: false,
        }
    }

    /// Stroke color. Default is `Color32::TRANSPARENT` which means a color will be auto-assigned.
    #[inline]
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.color = color.into();
        self
    }

    /// Stroke color.
    #[inline]
    pub fn get_color(&self) -> Color32 {
        self.color
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
        &self,
        pixel_width: usize,
        x_bounds: (f64, f64),
    ) -> (usize, Option<(usize, usize)>) {
        self.mipmap_min.get_level_match(pixel_width, x_bounds)
    }

    /// Apply an offset to the plot based on the difference to the supplied [`DateTime<Utc>`]
    pub fn offset_plot(&mut self, new_start_date: DateTime<Utc>) {
        util::offset_data_iter(self.raw_plot.iter_mut(), new_start_date);
        self.recalc_mipmaps();
    }

    fn recalc_mipmaps(&mut self) {
        self.mipmap_min = MipMap2D::without_base(
            &self.raw_plot,
            MipMapStrategy::Min,
            Self::MIPMAP_MIN_ELEMENTS,
        );
        self.mipmap_max = MipMap2D::without_base(
            &self.raw_plot,
            MipMapStrategy::Max,
            Self::MIPMAP_MIN_ELEMENTS,
        );
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
    pub fn log_id(&self) -> u16 {
        self.log_id
    }

    /// Label of the plot which includes the log id ie. `"<name> #<log_id>"`
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether or not the line should be highlighted
    pub fn get_highlight(&self) -> bool {
        self.highlight
    }

    /// Mutable reference to whether or not the line should be highlighted
    pub fn get_highlight_mut(&mut self) -> &mut bool {
        &mut self.highlight
    }
}

/// Represents all the plotlabels from a given log
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct StoredPlotLabels {
    pub log_id: u16,
    pub label_points: Vec<PlotLabel>,
    pub highlight: bool,
}

impl StoredPlotLabels {
    pub fn new(label_points: Vec<([f64; 2], String)>, log_id: u16) -> Self {
        Self {
            label_points: label_points.into_iter().map(PlotLabel::from).collect(),
            log_id,
            highlight: false,
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

    pub fn log_id(&self) -> u16 {
        self.log_id
    }

    /// Whether or not the labels should be highlighted
    pub fn get_highlight(&self) -> bool {
        self.highlight
    }

    /// Mutable reference to whether or not the labels should be highlighted
    pub fn get_highlight_mut(&mut self) -> &mut bool {
        &mut self.highlight
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
