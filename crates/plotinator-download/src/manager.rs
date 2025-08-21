use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use crate::{DownloadMessage, downloader, endpoint::Endpoint};

pub struct DownloadManager {
    tx: Sender<DownloadMessage>,
    rx: Receiver<DownloadMessage>,
    in_progress: bool,
    progress: f32,
    status_text: String,
}

impl DownloadManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            in_progress: false,
            progress: 0.0,
            status_text: String::new(),
        }
    }

    pub fn start_download(&mut self, host: String, port: String, endpoint: Endpoint) {
        if self.in_progress {
            return;
        }
        self.in_progress = true;
        self.progress = 0.0;
        self.status_text = "Connecting...".to_string();

        let tx = self.tx.clone();
        thread::Builder::new()
            .name("downloader".into())
            .spawn(move || {
                let result = downloader::download_zip(&host, &port, tx.clone(), endpoint);
                match result {
                    Ok(filename) => {
                        let _ = tx.send(DownloadMessage::Success { filename });
                    }
                    Err(e) => {
                        let _ = tx.send(DownloadMessage::Error(e.to_string()));
                    }
                }
                let _ = tx.send(DownloadMessage::Finished);
            })
            .expect("Failed spawning download thread");
    }

    pub fn poll(&mut self) -> Vec<DownloadMessage> {
        let mut messages = Vec::new();
        while let Ok(msg) = self.rx.try_recv() {
            if matches!(msg, DownloadMessage::Finished) {
                self.in_progress = false;
            }
            messages.push(msg);
        }
        messages
    }

    pub fn in_progress(&self) -> bool {
        self.in_progress
    }

    pub fn progress(&self) -> f32 {
        self.progress
    }

    pub fn update_progress(&mut self, downloaded: u64, total: u64) {
        if total > 0 {
            self.progress = downloaded as f32 / total as f32;
        }
        self.status_text = format!(
            "{} / {}",
            format_data_size(downloaded as usize),
            format_data_size(total as usize)
        );
    }

    pub fn status_text(&self) -> &str {
        &self.status_text
    }
}

/// Format a value to a human readable byte magnitude description
#[must_use]
pub fn format_data_size(size_bytes: usize) -> String {
    const KI_B_VAL: usize = 1024;
    const KI_B_DIVIDER: f64 = 1024_f64;
    const MI_B_VAL: usize = 1024 * KI_B_VAL;
    const MI_B_DIVIDER: f64 = MI_B_VAL as f64;
    const GI_B_VAL: usize = 1024 * MI_B_VAL;
    const GI_B_DIVIDER: f64 = GI_B_VAL as f64;
    match size_bytes {
        0..=KI_B_VAL => {
            format!("{size_bytes:.2} B")
        }
        1025..=MI_B_VAL => {
            let kib_bytes = size_bytes as f64 / KI_B_DIVIDER;
            format!("{kib_bytes:.2} KiB")
        }
        1_048_577..=GI_B_VAL => {
            let mib_bytes = size_bytes as f64 / MI_B_DIVIDER;
            format!("{mib_bytes:.2} MiB")
        }
        _ => {
            let gib_bytes = size_bytes as f64 / GI_B_DIVIDER;
            format!("{gib_bytes:.2} GiB")
        }
    }
}
