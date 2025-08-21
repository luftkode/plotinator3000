use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::bail;

use crate::app::DownloadMessage;

pub(crate) const ENDPOINT_DOWNLOAD_LATEST: &str = "/api/download/latest";
pub(crate) const ENDPOINT_DOWNLOAD_TODAY: &str = "/api/download/today";

/// Downloads a zip file from the specified host and port.
/// Returns the filename of the downloaded file on success.
pub fn download_zip(
    host: &str,
    port: &str,
    tx: Sender<DownloadMessage>,
    endpoint: &str,
) -> anyhow::Result<String> {
    // Construct the URL
    let url = format!("http://{host}:{port}{endpoint}");

    // Use a client with very long timeouts for zip generation
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30 * 60)) // 30 minutes total timeout
        .connect_timeout(Duration::from_secs(30)) // 30 seconds to establish connection
        .build()?;

    let mut response = client.get(&url).send()?;

    // Check if the response is successful
    if !response.status().is_success() {
        bail!("Server returned status: {}", response.status());
    }

    // Get total size from Content-Length header for the progress bar
    let total_size = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    log::info!("Receiving {total_size} bytes");

    // Create the plotinator directory in the downloads folder
    let download_dir = get_download_directory();
    let plotinator_dir = download_dir.join("plotinator");
    fs::create_dir_all(&plotinator_dir)?;

    // Generate a filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("plotinator_download_{}.zip", timestamp);
    let file_path = plotinator_dir.join(&filename);

    // Create the file and prepare for chunked writing
    let mut file = File::create(&file_path)?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0; 8192]; // 8KB buffer

    // Read the response in chunks
    while let Ok(bytes_read) = response.read(&mut buffer) {
        log::trace!("Read chunk: {bytes_read} bytes");
        if bytes_read == 0 {
            break; // End of stream
        }

        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

        // Send progress update to the UI thread
        tx.send(DownloadMessage::Progress {
            downloaded_bytes: downloaded,
            total_bytes: total_size,
        })?;
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
