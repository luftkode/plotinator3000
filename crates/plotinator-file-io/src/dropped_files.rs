use std::path::PathBuf;

use smallvec::{SmallVec, smallvec};

pub mod preview_dropped;

pub fn take_dropped_files(ctx: &egui::Context) -> SmallVec<[PathBuf; 1]> {
    preview_dropped::preview_files(ctx);

    let mut files = smallvec![];
    if let Some(dropped_files) = ctx.input(|in_state| {
        if in_state.raw.dropped_files.is_empty() {
            None
        } else {
            Some(in_state.raw.dropped_files.clone())
        }
    }) {
        for df in dropped_files {
            df.path.map(|p| files.push(p));
        }
    }

    files
}
