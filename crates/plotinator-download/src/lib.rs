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
