pub mod mipmap;

use egui_plot::{Line, PlotBounds, PlotPoints};
use log_if::LogEntry;
use mipmap::MipMap1D;
use serde::{Deserialize, Serialize};

pub type RawPlot = (Vec<[f64; 2]>, String, ExpectedPlotRange);

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct PlotWithName {
    pub raw_plot: Vec<[f64; 2]>,
    pub name: String,
}

impl PlotWithName {
    pub fn new(raw_plot: Vec<[f64; 2]>, name: String) -> Self {
        Self { raw_plot, name }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct MipMapWithName {
    pub mip_map: MipMap1D<f64>,
    pub name: String,
}

impl MipMapWithName {
    pub fn new(raw_y: Vec<f64>, name: String) -> Self {
        let mip_map = MipMap1D::new(raw_y);
        Self { mip_map, name }
    }
}

pub fn line_from_log_entry<XF, YF, L: LogEntry>(log: &[L], x_extractor: XF, y_extractor: YF) -> Line
where
    XF: Fn(&L) -> f64,
    YF: Fn(&L) -> f64,
{
    let points: PlotPoints = log
        .iter()
        .map(|e| [x_extractor(e), y_extractor(e)])
        .collect();
    Line::new(points)
}

pub fn raw_plot_from_log_entry<XF, YF, L: LogEntry>(
    log: &[L],
    x_extractor: XF,
    y_extractor: YF,
) -> Vec<[f64; 2]>
where
    XF: Fn(&L) -> f64,
    YF: Fn(&L) -> f64,
{
    log.iter()
        .map(|e| [x_extractor(e), y_extractor(e)])
        .collect()
}

pub fn raw_plot_from_normalized_timestamp<F, L: LogEntry>(
    log: &[L],
    normalized_timestamps_ms: &[f64],
    y_extractor: F,
) -> Vec<[f64; 2]>
where
    F: Fn(&L) -> f64,
{
    log.iter()
        .zip(normalized_timestamps_ms)
        .map(|(e, ts)| [*ts, y_extractor(e)])
        .collect()
}

/// Where does the plot values typically fit within
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Copy)]
pub enum ExpectedPlotRange {
    /// For plots where the value is 0.0-1.0 and corresponds to percentage 0-100%
    Percentage,
    OneToOneHundred,
    Thousands,
}

pub fn plot_lines(plot_ui: &mut egui_plot::PlotUi, plots: &[PlotWithName], line_width: f32) {
    for plot_with_name in plots {
        let x_min_max_ext = extended_x_plot_bound(plot_ui.plot_bounds(), 0.1);
        let filtered_points = filter_plot_points(&plot_with_name.raw_plot, x_min_max_ext);

        let line = Line::new(filtered_points).name(plot_with_name.name.clone());
        plot_ui.line(line.width(line_width));
    }
}

fn x_plot_bound(bounds: PlotBounds) -> (f64, f64) {
    let range = bounds.range_x();
    (*range.start(), *range.end())
}

/// Extends the x plot bounds by a specified percentage in both directions
pub fn extended_x_plot_bound(bounds: PlotBounds, extension_percentage: f64) -> (f64, f64) {
    let (x_bound_min, x_bound_max) = x_plot_bound(bounds);

    // Calculate the extension values based on the magnitude of the bounds
    let x_extension = (x_bound_max - x_bound_min).abs() * extension_percentage;

    // Extend the bounds
    let extended_x_bound_min = x_bound_min - x_extension;
    let extended_x_bound_max = x_bound_max + x_extension;

    (extended_x_bound_min, extended_x_bound_max)
}

#[inline(always)]
fn point_within(point: f64, bounds: (f64, f64)) -> bool {
    let (min, max) = bounds;
    min < point && point < max
}

/// Filter plot points based on the x plot bounds. Always includes the first and last plot point
/// such that resetting zooms works well even when the plot bounds are outside the data range.
pub fn filter_plot_points(points: &[[f64; 2]], x_range: (f64, f64)) -> Vec<[f64; 2]> {
    if points.is_empty() {
        return Vec::new();
    }

    let mut filtered = Vec::with_capacity(points.len());

    // Always include the first point
    filtered.push(points[0]);

    // Filter points within the extended range
    filtered.extend(
        points
            .iter()
            .skip(1)
            .take(points.len() - 2)
            .filter(|point| point_within(point[0], x_range))
            .copied(),
    );

    // Always include the last point if it's different from the first point
    if let Some(last_point) = points.last() {
        if *last_point != filtered[0] {
            filtered.push(*last_point);
        }
    }

    filtered
}
