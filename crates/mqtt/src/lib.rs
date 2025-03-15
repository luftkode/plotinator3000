use egui_plot::PlotPoint;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod broker_validator;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod mqtt_listener;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod topic_discoverer;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod util;

#[cfg(not(target_arch = "wasm32"))]
pub mod data_receiver;
#[cfg(not(target_arch = "wasm32"))]
pub mod mqtt_cfg_window;

/// Accumulated plot points from an MQTT topic
#[derive(Debug)]
pub struct MqttPoints {
    pub topic: String,
    pub data: Vec<PlotPoint>,
}

/// A single plot point with its topic origin
#[derive(Debug)]
pub struct MqttPoint {
    pub topic: String,
    pub point: PlotPoint,
}
