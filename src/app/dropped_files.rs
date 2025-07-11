use std::io;

use crate::{
    app::{
        file_dialog::{MagicFileContent, try_parse_magic_fil_from_buf, try_parse_magic_file},
        loaded_files::LoadedFiles,
    },
    plot::LogPlotUi,
};

pub mod preview_dropped;

pub(crate) fn handle_dropped_files(
    ctx: &egui::Context,
    loaded_files: &mut LoadedFiles,
) -> io::Result<Option<Box<LogPlotUi>>> {
    preview_dropped::preview_files(ctx);
    if let Some(dropped_files) = ctx.input(|in_state| {
        if in_state.raw.dropped_files.is_empty() {
            None
        } else {
            Some(in_state.raw.dropped_files.clone())
        }
    }) {
        for dfile in dropped_files {
            if let Some(content) = dfile.bytes.as_ref() {
                match try_parse_magic_fil_from_buf(content) {
                    Some(MagicFileContent::PlotData(plot_data)) => {
                        log::info!(
                            "Loading {} plot data files from web contents",
                            plot_data.len()
                        );
                        loaded_files.loaded.extend(plot_data);
                    }
                    Some(MagicFileContent::PlotUi(plot_ui)) => {
                        log::info!("Loading plot UI state from web contents");
                        return Ok(Some(plot_ui));
                    }
                    None => {
                        log::info!("Parsing regular file from web contents");
                        loaded_files.parse_raw_buffer(content)?;
                    }
                }
            } else if let Some(path) = &dfile.path {
                match try_parse_magic_file(path)? {
                    Some(MagicFileContent::PlotData(plot_data)) => {
                        log::info!("Loading {} plot data files from {path:?}", plot_data.len());
                        loaded_files.loaded.extend(plot_data);
                    }
                    Some(MagicFileContent::PlotUi(plot_ui)) => {
                        log::info!("Loading plot UI state from {path:?}");
                        return Ok(Some(plot_ui));
                    }
                    None => {
                        log::info!("Parsing regular file: {path:?}");
                        loaded_files.parse_path(path)?;
                    }
                }
            }
        }
    }
    Ok(None)
}
