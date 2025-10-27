use chrono::{DateTime, Utc};
use plotinator_log_if::prelude::*;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SerializableMqttPlotData {
    pub(crate) descriptive_name: String,
    pub(crate) first_timestamp: DateTime<Utc>,
    pub(crate) mqtt_plot_data: Vec<RawPlot>,
}

// A helper struct that *can* derive Serialize and Deserialize
// It represents the data in a way that Serde understands for PlotPoint which does not itself implement serialize/deserialize
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SerializableMqttPlotPoints {
    pub(crate) legend_name: String,
    pub(crate) topic: String,
    pub(crate) data: Vec<[f64; 2]>, // Represent PlotPoint as a tuple for serialization
    pub(crate) ty: Option<DataType>,
}

impl Plotable for SerializableMqttPlotData {
    fn raw_plots(&self) -> &[RawPlot] {
        &self.mqtt_plot_data
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.first_timestamp
    }

    fn descriptive_name(&self) -> &str {
        &self.descriptive_name
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        None
    }
}
