use std::{io, path::PathBuf};

use crate::app::loaded_files::LoadedFiles;

#[derive(Debug, Default)]
pub struct NativeFileDialog {
    picked_files: Vec<PathBuf>,
}

impl NativeFileDialog {
    pub(crate) fn open(&mut self) {
        if let Some(paths) = rfd::FileDialog::new().pick_files() {
            for p in paths {
                self.picked_files.push(p);
            }
        }
    }

    pub(crate) fn parse_picked_files(&mut self, loaded_files: &mut LoadedFiles) -> io::Result<()> {
        for pf in self.picked_files.drain(..) {
            loaded_files.parse_path(&pf)?;
        }
        Ok(())
    }
}
