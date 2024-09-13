use egui_plot::{Line, PlotPoints};

use crate::logs::LogEntry;

use super::mipmap::MipMap1D;

pub type RawPlot = (Vec<[f64; 2]>, String, ExpectedPlotRange);

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlotWithName {
    pub raw_plot: Vec<[f64; 2]>,
    pub name: String,
}

impl PlotWithName {
    pub fn new(raw_plot: Vec<[f64; 2]>, name: String) -> Self {
        Self { raw_plot, name }
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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

/// Where does the plot values typically fit within
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ExpectedPlotRange {
    /// For plots where the value is 0.0-1.0 and corresponds to percentage 0-100%
    Percentage,
    OneToOneHundred,
    Thousands,
}
