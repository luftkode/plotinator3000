use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct PlotData {
    plots: Vec<PlotWithName>,
    plot_labels: Vec<StoredPlotLabels>,
}

impl PlotData {
    pub fn plots(&self) -> &[PlotWithName] {
        &self.plots
    }

    pub fn plots_as_mut(&mut self) -> &mut Vec<PlotWithName> {
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
pub struct PlotWithName {
    pub raw_plot: Vec<[f64; 2]>,
    pub name: String,
    pub log_id: String,
}

impl PlotWithName {
    pub fn new(raw_plot: Vec<[f64; 2]>, name: String, id: String) -> Self {
        Self {
            raw_plot,
            name,
            log_id: id,
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct StoredPlotLabels {
    pub label_points: Vec<PlotLabel>,
    pub log_id: String,
}

impl StoredPlotLabels {
    pub fn new(label_points: Vec<([f64; 2], String)>, id: String) -> Self {
        Self {
            label_points: label_points.into_iter().map(|v| v.into()).collect(),
            log_id: id,
        }
    }

    pub fn labels(&self) -> &[PlotLabel] {
        &self.label_points
    }

    pub fn label_points_mut(&mut self) -> impl Iterator<Item = &mut [f64; 2]> {
        self.label_points.iter_mut().map(|label| &mut label.point)
    }

    pub fn log_id(&self) -> &str {
        &self.log_id
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
