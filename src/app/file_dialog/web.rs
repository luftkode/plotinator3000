use std::{
    io,
    sync::mpsc::{Receiver, Sender, channel},
};

use plotinator_supported_formats::SupportedFormat;
use serde::Serialize;

use crate::{
    app::{
        LoadedFiles,
        custom_files::{
            CUSTOM_HEADER_PLOT_DATA, CUSTOM_HEADER_PLOT_UI_STATE, CustomFileContent,
            try_parse_custom_file_from_buf,
        },
        file_dialog::{FILE_FILTER_EXTENSIONS, FILE_FILTER_NAME},
    },
    plot::LogPlotUi,
};

fn execute<F: std::future::Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

struct WebFileContents {
    name: String,
    contents: Vec<u8>,
}

#[derive(Debug)]
pub struct WebFileDialog {
    file_sender: Sender<WebFileContents>,
    file_receiver: Receiver<WebFileContents>,
}

impl Default for WebFileDialog {
    fn default() -> Self {
        let (file_sender, file_receiver) = channel();
        Self {
            file_sender,
            file_receiver,
        }
    }
}

impl WebFileDialog {
    pub(crate) fn open(&self, ctx: egui::Context) {
        Self::open_dialog(ctx, self.file_sender.clone());
    }

    /// Saves the plot UI state to a file.
    pub(crate) fn save_plot_ui(plot_ui: &LogPlotUi) {
        Self::save_data_to_file(
            plot_ui,
            "Save Plot UI State",
            "plotinator3k_plotui.p3k",
            CUSTOM_HEADER_PLOT_UI_STATE,
        );
    }

    /// Saves the plot data to a file.
    pub(crate) fn save_plot_data(plot_files: &[SupportedFormat]) {
        if !plot_files.is_empty() {
            Self::save_data_to_file(
                plot_files,
                "Save Plot Data",
                "plotinator3k.p3k",
                CUSTOM_HEADER_PLOT_DATA,
            );
        }
    }

    /// Generic function to save serializable data with a custom header for web.
    fn save_data_to_file<T: Serialize + ?Sized>(
        data: &T,
        title: &str,
        default_file_name: &str,
        custom_header: &str,
    ) {
        let serialized_data = match serde_json::to_string(data) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to serialize data: {e}");
                return;
            }
        };

        let mut contents = String::with_capacity(serialized_data.len() + custom_header.len());
        contents.push_str(custom_header);
        contents.push_str(&serialized_data);
        let contents_bytes = contents.into_bytes();

        let task = rfd::AsyncFileDialog::new()
            .set_title(title)
            .set_file_name(default_file_name)
            .save_file();

        execute(async move {
            if let Some(file_handle) = task.await {
                if let Err(e) = file_handle.write(&contents_bytes).await {
                    log::error!("Failed to write to file: {e}");
                }
            }
        });
    }

    pub(crate) fn poll_received_files(
        &self,
        loaded_files: &mut LoadedFiles,
    ) -> io::Result<Option<Box<LogPlotUi>>> {
        if let Ok(file_web_content) = self.file_receiver.try_recv() {
            log::debug!("Received file: {}", file_web_content.name);
            let raw_contents = &file_web_content.contents;
            match try_parse_custom_file_from_buf(raw_contents) {
                Some(CustomFileContent::PlotData(plot_data)) => {
                    log::info!(
                        "Loading {} plot data files from web contents",
                        plot_data.len()
                    );
                    loaded_files.loaded.extend(plot_data);
                }
                Some(CustomFileContent::PlotUi(plot_ui)) => {
                    log::info!("Loading plot UI state from web contents");
                    return Ok(Some(plot_ui));
                }
                None => {
                    log::info!("Parsing regular file from web contents");
                    loaded_files.parse_raw_buffer(raw_contents)?;
                }
            }
        }
        Ok(None)
    }

    fn open_dialog(ctx: egui::Context, sender: Sender<WebFileContents>) {
        let task = rfd::AsyncFileDialog::new()
            .add_filter(FILE_FILTER_NAME, FILE_FILTER_EXTENSIONS)
            .pick_files();

        execute(async move {
            let files = task.await;
            if let Some(files) = files {
                for f in files {
                    let name = f.file_name();
                    let fwebcontents = WebFileContents {
                        name,
                        contents: f.read().await,
                    };
                    let _ = sender.send(fwebcontents);
                    ctx.request_repaint();
                }
            }
        });
    }
}
