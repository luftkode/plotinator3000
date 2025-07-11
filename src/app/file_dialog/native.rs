use std::{fs, io, path::PathBuf};

use crate::{
    app::{
        custom_files::{
            CUSTOM_HEADER_PLOT_DATA, CUSTOM_HEADER_PLOT_UI_STATE, CustomFileContent,
            try_parse_custom_file,
        },
        file_dialog::{FILE_FILTER_EXTENSIONS, FILE_FILTER_NAME},
        loaded_files::LoadedFiles,
    },
    plot::LogPlotUi,
};
use plotinator_supported_formats::SupportedFormat;
use serde::Serialize;

#[derive(Debug, Default)]
pub struct NativeFileDialog {
    picked_files: Vec<PathBuf>,
}

impl NativeFileDialog {
    /// Opens a native file dialog to pick multiple files.
    pub(crate) fn open(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter(FILE_FILTER_NAME, FILE_FILTER_EXTENSIONS)
            .pick_files()
        {
            self.picked_files.extend(paths);
        }
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
    pub(crate) fn save_plot_data(
        plot_files: &[SupportedFormat],
        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))] mqtt_plots: Option<
            &plotinator_mqtt::MqttPlotData,
        >,
    ) {
        let title = "Save Plot Data";

        if !plot_files.is_empty() {
            Self::save_data_to_file(
                plot_files,
                title,
                "plotinator3k.p3k",
                CUSTOM_HEADER_PLOT_DATA,
            );
            return;
        }

        #[cfg(all(not(target_arch = "wasm32"), feature = "mqtt"))]
        if mqtt_plots.is_some() {
            if let Some(mqtt_plot_data) = mqtt_plots {
                let supported_formats: Vec<SupportedFormat> =
                    vec![SupportedFormat::MqttData(mqtt_plot_data.clone().into())];
                Self::save_data_to_file(
                    &supported_formats,
                    title,
                    "mqtt_potinator3k.p3k",
                    CUSTOM_HEADER_PLOT_DATA,
                );
            }
        }
    }

    /// Generic function to save serializable data with a custom header.
    fn save_data_to_file<T: Serialize + ?Sized>(
        data: &T,
        title: &str,
        default_file_name: &str,
        custom_header: &str,
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
                        String::with_capacity(serialized_data.len() + custom_header.len());
                    contents.push_str(custom_header);
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
        for pf in self.picked_files.drain(..) {
            match try_parse_custom_file(&pf)? {
                Some(CustomFileContent::PlotData(plot_data)) => {
                    log::info!("Loading {} plot data files from {pf:?}", plot_data.len());
                    loaded_files.loaded.extend(plot_data);
                }
                Some(CustomFileContent::PlotUi(plot_ui)) => {
                    log::info!("Loading plot UI state from {pf:?}");
                    return Ok(Some(plot_ui));
                }
                None => {
                    log::info!("Parsing regular file: {pf:?}");
                    loaded_files.parse_path(&pf)?;
                }
            }
        }
        Ok(None)
    }
}
