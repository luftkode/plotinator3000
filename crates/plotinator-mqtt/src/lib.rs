use plotinator_macros::non_wasm_modules;
non_wasm_modules!(
    pub mod broker_validator;
    pub(crate) mod mqtt_listener;
    pub(crate) mod topic_discoverer;
    pub(crate) mod parse_packet;
    pub(crate) mod util;
    pub mod data_receiver;
    pub mod mqtt_cfg_window;
    pub mod data;
    pub(crate) mod ui;
);
#[cfg(not(target_arch = "wasm32"))]
pub use crate::{
    broker_validator::BrokerStatus, data::plot::MqttPlotPoints, data_receiver::MqttDataReceiver,
    mqtt_cfg_window::MqttConfigWindow,
};
