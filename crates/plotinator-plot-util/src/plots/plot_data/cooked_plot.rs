use std::ops::RangeInclusive;

use chrono::{DateTime, Utc};
use egui::Color32;
use egui_plot::{PlotBounds, PlotPoint};
use plotinator_log_if::prelude::*;
use plotinator_ui_util::{ExpectedPlotRange, auto_color_plot_area};
use serde::{Deserialize, Serialize};

use crate::mipmap::{MipMap2DPlotPoints, MipMapStrategy};

use super::util;

#[derive(Deserialize, Serialize)]
pub struct CookedPlot {
    raw_points: Vec<[f64; 2]>,
    #[serde(skip)]
    raw_plot_points: Option<Vec<PlotPoint>>,
    #[serde(skip)]
    mipmap_minmax_plot_points: Option<MipMap2DPlotPoints>,
    #[serde(skip)]
    max_bounds: Option<PlotBounds>,
    name: String,
    log_id: u16,
    // Label = "<name> #<log_id>"
    label: String,
    // The descriptive name of the log that this data set originated from
    associated_descriptive_name: String,
    color: Color32,
    highlight: bool,
    ty: DataType,
    expected_range: ExpectedPlotRange,
}

type PointList<'pl> = &'pl [[f64; 2]];

impl CookedPlot {
    // Don't mipmap/downsample to more than this amount of elements
    const MIPMAP_MIN_ELEMENTS: usize = 512;

    #[plotinator_proc_macros::log_time]
    pub fn new(raw_plot: &RawPlotCommon, log_id: u16, associated_descriptive_name: String) -> Self {
        plotinator_macros::profile_function!();
        let label = raw_plot.label_from_id(log_id);
        let raw_plot_points = Some(raw_plot.points().iter().map(|p| (*p).into()).collect());

        let raw_points = raw_plot.points().to_owned();

        let mipmap_max_pp = MipMap2DPlotPoints::without_base(
            &raw_points,
            MipMapStrategy::Max,
            Self::MIPMAP_MIN_ELEMENTS,
        );
        let mut mipmap_min_pp = MipMap2DPlotPoints::without_base(
            &raw_points,
            MipMapStrategy::Min,
            Self::MIPMAP_MIN_ELEMENTS,
        );
        mipmap_min_pp.join(&mipmap_max_pp);
        let max_bounds = Self::calc_max_bounds(&raw_points, mipmap_min_pp.get_max_level());

        let color = match raw_plot.color() {
            Some(c) => c,
            None => auto_color_plot_area(raw_plot.expected_range()),
        };

        Self {
            raw_points,
            raw_plot_points,
            mipmap_minmax_plot_points: Some(mipmap_min_pp),
            max_bounds: Some(max_bounds),
            name: raw_plot.legend_name().to_owned(),
            log_id,
            ty: raw_plot.ty().clone(),
            label,
            associated_descriptive_name,
            color,
            highlight: false,
            expected_range: raw_plot.expected_range(),
        }
    }

    /// Stroke color.
    #[inline]
    pub fn get_color(&self) -> Color32 {
        self.color
    }

    pub fn get_raw(&self) -> PointList<'_> {
        &self.raw_points
    }

    pub fn get_level(&self, level: usize) -> Option<&[PlotPoint]> {
        self.mipmap_minmax_plot_points
            .as_ref()
            .expect("Accessed mipmaps before they were populated (missing call to 'build_raw_plot_points'?)")
            .get_level(level)
    }

    pub fn get_level_or_max(&self, level: usize) -> &[PlotPoint] {
        self.mipmap_minmax_plot_points
            .as_ref()
            .expect("Accessed mipmaps before they were populated (missing call to 'build_raw_plot_points'?)")
            .get_level_or_max(level)
    }

    pub fn get_max_level(&self) -> &[PlotPoint] {
        self.mipmap_minmax_plot_points
            .as_ref()
            .expect("Accessed mipmaps before they were populated (missing call to 'build_raw_plot_points'?)")
            .get_max_level()
    }

    pub fn mipmap_levels(&self) -> usize {
        self.mipmap_minmax_plot_points
            .as_ref()
            .expect("Accessed mipmaps before they were populated (missing call to 'build_raw_plot_points'?)")
            .num_levels()
    }

    pub fn get_scaled_mipmap_levels(
        &self,
        pixel_width: usize,
        x_bounds: RangeInclusive<f64>,
    ) -> (usize, Option<(usize, usize)>) {
        self.mipmap_minmax_plot_points
            .as_ref()
            .expect("Accessed mipmaps before they were populated (missing call to 'build_raw_plot_points'?)")
            .get_level_match(pixel_width, x_bounds)
    }

    /// Apply an offset to the plot based on the difference to the supplied [`DateTime<Utc>`]
    pub fn offset_plot(&mut self, new_start_date: DateTime<Utc>) {
        util::offset_data_iter(self.raw_points.iter_mut(), new_start_date);
        self.raw_plot_points = Some(self.raw_points.iter().map(|p| (*p).into()).collect());
        self.recalc_mipmaps_plot_points();
    }

    /// Remove all points between `start` and `end`
    pub fn cut_plot_within_x_range(&mut self, start: f64, end: f64) {
        log::info!("Removing points within range: {start} - {end}");
        let begin_count = self.raw_points.len();

        self.raw_points.retain(|p| p[0] < start || p[0] > end);

        let end_count = self.raw_points.len();
        let removed_count = begin_count - end_count;
        log::info!("Removed {removed_count} points");
        self.raw_plot_points = Some(self.raw_points.iter().map(|p| (*p).into()).collect());
        self.recalc_mipmaps_plot_points();
    }

    /// Remove points with x ∈ [start, end] but y ∉ [min, max]
    pub fn cut_plot_outside_minmax(&mut self, start: f64, end: f64, min: f64, max: f64) {
        log::info!("Removing points outside min-max: {min:.2} - {max:.2}");
        let begin_count = self.raw_points.len();

        self.raw_points.retain(|p|
            // outside the x-range: always keep
        if p[0] < start || p[0] > end {
            true
        } else {
            // inside x-range: keep only if y is within [min, max]
            p[1] >= min && p[1] <= max
        });

        let end_count = self.raw_points.len();
        let removed_count = begin_count - end_count;
        log::info!("Removed {removed_count} points");
        self.raw_plot_points = Some(self.raw_points.iter().map(|p| (*p).into()).collect());
        self.recalc_mipmaps_plot_points();
    }

    fn recalc_mipmaps_plot_points(&mut self) {
        let mipmap_max_pp = MipMap2DPlotPoints::without_base(
            &self.raw_points,
            MipMapStrategy::Max,
            Self::MIPMAP_MIN_ELEMENTS,
        );
        let mut mipmap_min_pp = MipMap2DPlotPoints::without_base(
            &self.raw_points,
            MipMapStrategy::Min,
            Self::MIPMAP_MIN_ELEMENTS,
        );
        mipmap_min_pp.join(&mipmap_max_pp);
        self.max_bounds = Some(Self::calc_max_bounds(
            &self.raw_points,
            mipmap_min_pp.get_max_level(),
        ));
        self.mipmap_minmax_plot_points = Some(mipmap_min_pp);
    }

    fn calc_max_bounds(raw_plot: &[[f64; 2]], mipmap_joined_max_lvl: &[PlotPoint]) -> PlotBounds {
        let first_x = raw_plot
            .first()
            .and_then(|f| f.first())
            .expect("Empty dataset");
        let last_x = raw_plot
            .last()
            .and_then(|f| f.first())
            .expect("Empty dataset");

        // We unwrap the partial_cmp in such a way that any number beats NaN in the comparison.
        let min_y = mipmap_joined_max_lvl
            .iter()
            .min_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Less))
            .expect("Empty slice, we should've checked that earlier")
            .y;
        let max_y = mipmap_joined_max_lvl
            .iter()
            .max_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Greater))
            .expect("Empty slice, we should've checked that earlier")
            .y;
        PlotBounds::from_min_max([*first_x, min_y], [*last_x, max_y])
    }

    pub fn total_data_points(&self) -> usize {
        self.raw_points.len()
    }

    pub fn first_timestamp(&self) -> f64 {
        *self
            .raw_points
            .first()
            .and_then(|f| f.first())
            .expect("Empty dataset")
    }

    pub fn last_timestamp(&self) -> f64 {
        *self
            .raw_points
            .last()
            .and_then(|f| f.first())
            .expect("Empty dataset")
    }

    /// Generates the raw plot points from the `raw_points` (only needed once per session)
    ///
    /// necessary because the raw plot points are not serializable
    /// so they are skipped and initialized as None at start up.
    pub fn build_raw_plot_points(&mut self) {
        if self.raw_plot_points.is_none() {
            self.raw_plot_points = Some(self.raw_points.iter().map(|p| (*p).into()).collect());
            self.recalc_mipmaps_plot_points();
        }
    }

    pub fn raw_plot_points(&self) -> &[PlotPoint] {
        self.raw_plot_points
            .as_deref()
            .expect("Attempted to retrieve raw plot points without first generating it")
    }

    /// The descriptive name of the log the plot values are associated with, e.g. `Navsys` or `frame-altimeter`
    pub fn associated_descriptive_name(&self) -> &str {
        &self.associated_descriptive_name
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

    pub fn ty(&self) -> &DataType {
        &self.ty
    }

    /// Whether or not the line should be highlighted
    pub fn get_highlight(&self) -> bool {
        self.highlight
    }

    /// Mutable reference to whether or not the line should be highlighted
    pub fn get_highlight_mut(&mut self) -> &mut bool {
        &mut self.highlight
    }

    pub fn get_max_bounds(&self) -> PlotBounds {
        self.max_bounds.expect("Accessed max_bounds before it was populated (missing call to 'build_raw_plot_points'?)")
    }
}
