use std::fs::{self, File};
use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use reqwest::Url;

use crate::DownloadMessage;
use crate::endpoint::Endpoint;

struct DownloadProgress {
    tx: Sender<DownloadMessage>,
    total_bytes: u64,
    bytes_received: u64,
}

impl DownloadProgress {
    pub fn new(tx: Sender<DownloadMessage>, total_size: u64) -> Self {
        tx.send(DownloadMessage::Progress {
            downloaded_bytes: 0,
            total_bytes: total_size,
        })
        .expect("Failed sending initial download progress message");
        Self {
            tx,
            total_bytes: total_size,
            bytes_received: 0,
        }
    }

    pub fn update(&mut self, bytes_received: u64) -> anyhow::Result<()> {
        self.bytes_received += bytes_received;
        self.tx.send(DownloadMessage::Progress {
            downloaded_bytes: self.bytes_received,
            total_bytes: self.total_bytes,
        })?;
        Ok(())
    }
}

/// Downloads a zip file from the specified host and port.
/// Returns the filename of the downloaded file on success.
pub(crate) fn download_zip(
    host: &str,
    port: &str,
    tx: Sender<DownloadMessage>,
    endpoint: Endpoint,
) -> anyhow::Result<String> {
    let url: Url = format!("http://{host}:{port}{endpoint}").parse()?;

    // Use a client with very long timeouts to allow time for zip generation
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10 * 60)) // 10 minutes total timeout
        .connect_timeout(Duration::from_secs(20)) // 20 seconds to establish connection
        .build()?;

    let mut response = client.get(url).send()?;

    anyhow::ensure!(
        response.status().is_success(),
        "Server did not respond with success (200-299 HTTP code)"
    );

    // Get total size from Content-Length header for the progress bar
    let total_size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    anyhow::ensure!(total_size > 0, "Request yielded empty file (size is 0)");

    log::info!("Receiving {total_size} bytes");

    let mut dl_progress = DownloadProgress::new(tx, total_size);

    // Create the plotinator directory in the downloads folder
    let download_dir = get_download_directory();
    let plotinator_dir = download_dir.join("plotinator");
    fs::create_dir_all(&plotinator_dir)?;

    // Generate a filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("plotinator_download_{timestamp}.zip");
    let file_path = plotinator_dir.join(&filename);

    // Create the file and prepare for chunked writing
    let mut file = File::create(&file_path)?;
    let mut buffer = [0; 8192]; // 8KB buffer

    // Read the response in chunks
    while let Ok(bytes_read) = response.read(&mut buffer) {
        log::trace!("Read chunk: {bytes_read} bytes");
        if bytes_read == 0 {
            break; // End of stream
        }

        file.write_all(&buffer[..bytes_read])?;

        dl_progress.update(bytes_read as u64)?;
    }

    file.flush()?;

    Ok(filename)
}

fn get_download_directory() -> PathBuf {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Downloads")))
        .unwrap_or_else(|| {
            let fallback = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("downloads");

            // Try to create the directory
            let _ = fs::create_dir_all(&fallback);
            fallback
        })
}
