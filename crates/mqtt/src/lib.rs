use plotinator_macros::non_wasm_modules;
non_wasm_modules!(
    pub(crate) mod broker_validator;
    pub(crate) mod mqtt_listener;
    pub(crate) mod topic_discoverer;
    pub(crate) mod known_topics;
    pub mod data_receiver;
    pub mod mqtt_cfg_window;
    pub mod data;
);
#[cfg(not(target_arch = "wasm32"))]
pub use crate::data::{MqttPoint, MqttPoints};
