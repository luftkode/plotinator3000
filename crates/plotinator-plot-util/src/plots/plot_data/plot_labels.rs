use chrono::{DateTime, Utc};
use plotinator_ui_util::ExpectedPlotRange;

/// Represents all the plotlabels from a given log
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StoredPlotLabels {
    pub log_id: u16,
    pub label_points: Vec<PlotLabel>,
    pub highlight: bool,
    pub expected_range: ExpectedPlotRange,
}

impl StoredPlotLabels {
    pub fn new(
        label_points: Vec<([f64; 2], String)>,
        log_id: u16,
        expected_range: ExpectedPlotRange,
    ) -> Self {
        Self {
            label_points: label_points.into_iter().map(PlotLabel::from).collect(),
            log_id,
            expected_range,
            highlight: false,
        }
    }

    pub fn labels(&self) -> &[PlotLabel] {
        &self.label_points
    }

    /// Apply an offset to the plot labels based on the difference to the supplied [`DateTime<Utc>`]
    pub fn offset_labels(&mut self, new_start_date: DateTime<Utc>) {
        crate::plots::util::offset_data_iter(self.label_points_mut(), new_start_date);
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

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
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
