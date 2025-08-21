#![cfg(not(target_arch = "wasm32"))]

// Strings because we use them as defaults in editable text boxes
pub const TS_IP: &str = "192.168.1.60";
pub const DATA_BINDER_PORT: &str = "9999";

pub enum DownloadMessage {
    Success {
        filename: String,
    },
    Error(String),
    Progress {
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    Finished,
}

pub(crate) mod downloader;
pub mod endpoint;
pub mod manager;
