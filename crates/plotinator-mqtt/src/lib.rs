plotinator_macros::non_wasm_modules!(
    pub mod broker_validator;
    pub mod topic_discoverer;
    pub(crate) mod parse_packet;
    pub(crate) mod util;
    pub mod data_receiver;
    pub mod data;
    pub mod client;
);

#[cfg(not(target_arch = "wasm32"))]
pub use crate::broker_validator::BrokerStatus;
