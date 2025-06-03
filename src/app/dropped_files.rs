use std::io;

use crate::app::loaded_files::LoadedFiles;

pub mod preview_dropped;

pub(crate) fn handle_dropped_files(
    ctx: &egui::Context,
    loaded_files: &mut LoadedFiles,
) -> io::Result<()> {
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
                loaded_files.parse_raw_buffer(content)?;
            } else if let Some(path) = &dfile.path {
                loaded_files.parse_path(path)?;
            }
        }
    }
    Ok(())
}
