use chrono::{DateTime, Utc};
use plotinator_log_if::prelude::{Plotable, RawPlot};
use plotinator_macros::non_wasm_modules;
use serde::{Deserialize, Serialize};
non_wasm_modules!(
    pub mod broker_validator;
    pub(crate) mod topic_discoverer;
    pub(crate) mod parse_packet;
    pub(crate) mod util;
    pub mod data_receiver;
    pub mod mqtt_cfg_window;
    pub mod data;
    pub(crate) mod ui;
    pub(crate) mod client;
);
#[cfg(not(target_arch = "wasm32"))]
pub use crate::{
    broker_validator::BrokerStatus, data::plot::MqttPlotData, data::plot::MqttPlotPoints,
    data_receiver::MqttDataReceiver, mqtt_cfg_window::MqttConfigWindow,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SerializableMqttPlotData {
    descriptive_name: String,
    first_timestamp: DateTime<Utc>,
    mqtt_plot_data: Vec<RawPlot>,
}

// A helper struct that *can* derive Serialize and Deserialize
// It represents the data in a way that Serde understands for PlotPoint which does not itself implement serialize/deserialize
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SerializableMqttPlotPoints {
    topic: String,
    data: Vec<[f64; 2]>, // Represent PlotPoint as a tuple for serialization
}

impl Plotable for SerializableMqttPlotData {
    fn raw_plots(&self) -> &[plotinator_log_if::prelude::RawPlot] {
        &self.mqtt_plot_data
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        self.first_timestamp
    }

    fn descriptive_name(&self) -> &str {
        &self.descriptive_name
    }

    fn labels(&self) -> Option<&[plotinator_log_if::prelude::PlotLabels]> {
        None
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        None
    }
}
