use std::{
    fs,
    io::{self, Read as _, Seek as _},
    path::PathBuf,
};

use crate::{
    app::{
        file_dialog::{
            MAGIC_HEADER_PLOT_DATA, MAGIC_HEADER_PLOT_UI_STATE, MagicFileContent,
            deserialize_magic_content_from_bytes, parse_magic_header_from_bytes,
        },
        loaded_files::LoadedFiles,
    },
    plot::LogPlotUi,
};
use plotinator_supported_formats::SupportedFormat;
use serde::Serialize; // Add this import for the generic save function

#[derive(Debug, Default)]
pub struct NativeFileDialog {
    picked_files: Vec<PathBuf>,
}

impl NativeFileDialog {
    /// Opens a native file dialog to pick multiple files.
    pub(crate) fn open(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().pick_files() {
            self.picked_files.extend(paths);
        }
    }

    /// Saves the plot UI state to a file.
    pub(crate) fn save_plot_ui(plot_ui: &LogPlotUi) {
        Self::save_data_to_file(
            plot_ui,
            "Save Plot UI State",
            "plotinator3k_plotui.state",
            MAGIC_HEADER_PLOT_UI_STATE,
        );
    }

    /// Saves the plot data to a file.
    pub(crate) fn save_plot_data(plot_files: &[SupportedFormat]) {
        if !plot_files.is_empty() {
            Self::save_data_to_file(
                plot_files,
                "Save Plot Data",
                "plotinator3k.data",
                MAGIC_HEADER_PLOT_DATA,
            );
        }
    }

    /// Generic function to save serializable data with a magic header.
    fn save_data_to_file<T: Serialize + ?Sized>(
        data: &T,
        title: &str,
        default_file_name: &str,
        magic_header: &str,
    ) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(title)
            .set_file_name(default_file_name)
            .save_file()
        {
            log::info!("Saving data to {path:?}");
            match serde_json::to_string(data) {
                Ok(serialized_data) => {
                    let mut contents =
                        String::with_capacity(serialized_data.len() + magic_header.len());
                    contents.push_str(magic_header);
                    contents.push_str(&serialized_data);
                    if let Err(e) = fs::write(&path, contents) {
                        log::error!("Failed to write to file {path:?}: {e}");
                    }
                }
                Err(e) => {
                    log::error!("Failed to serialize data: {e}");
                }
            }
        }
    }

    /// Parses all picked files and loads them into the application.
    /// Returns an `Option<LogPlotUi>` if a plot UI state file was loaded.
    pub(crate) fn parse_picked_files(
        &mut self,
        loaded_files: &mut LoadedFiles,
    ) -> io::Result<Option<Box<LogPlotUi>>> {
        let mut loaded_plot_ui = None;
        for pf in self.picked_files.drain(..) {
            match try_parse_magic_file(&pf)? {
                Some(MagicFileContent::PlotData(plot_data)) => {
                    log::info!("Loading {} plot data files from {pf:?}", plot_data.len());
                    loaded_files.loaded.extend(plot_data);
                }
                Some(MagicFileContent::PlotUi(plot_ui)) => {
                    log::info!("Loading plot UI state from {pf:?}");
                    loaded_plot_ui = Some(plot_ui);
                }
                None => {
                    log::info!("Parsing regular file: {pf:?}");
                    loaded_files.parse_path(&pf)?;
                }
            }
        }
        Ok(loaded_plot_ui)
    }
}

/// Attempts to parse a file that might contain a Plotinator3000 magic header.
fn try_parse_magic_file(path: &PathBuf) -> io::Result<Option<MagicFileContent>> {
    let mut file = fs::File::open(path)?;
    let max_header_len = MAGIC_HEADER_PLOT_DATA
        .len()
        .max(MAGIC_HEADER_PLOT_UI_STATE.len());
    let mut header_buf = vec![0u8; max_header_len];

    // Attempt to read the header. If it's too short, it's not a magic file.
    if file.read_exact(&mut header_buf).is_err() {
        return Ok(None);
    }

    let Some((magic_header_len, is_plot_ui)) = parse_magic_header_from_bytes(&header_buf) else {
        return Ok(None);
    };

    // Seek past the magic header to the actual data
    file.seek(io::SeekFrom::Start(magic_header_len as u64))?;

    let mut data_bytes = Vec::new();
    #[allow(clippy::verbose_file_reads, reason = "false positive?")]
    file.read_to_end(&mut data_bytes)?; // Read the rest of the file into bytes

    match deserialize_magic_content_from_bytes(&data_bytes, is_plot_ui) {
        Ok(content) => Ok(Some(content)),
        Err(e) => {
            log::error!("Failed to deserialize magic file content from {path:?}: {e}");
            Ok(None)
        }
    }
}
