use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self},
    path::Path,
};

use crate::app::supported_format::SupportedFormat;

/// Contains all supported logs in a single vector.
#[derive(Default, Deserialize, Serialize)]
pub struct LoadedFiles {
    loaded: Vec<SupportedFormat>,
}

impl LoadedFiles {
    /// Return a vector of immutable references to all logs
    pub(crate) fn loaded(&self) -> &[SupportedFormat] {
        &self.loaded
    }

    /// Take all the `loaded_files` currently stored and return them as a list
    pub(crate) fn take_loaded_files(&mut self) -> Vec<SupportedFormat> {
        self.loaded.drain(..).collect()
    }

    pub(crate) fn parse_path(&mut self, path: &Path) -> io::Result<()> {
        if path.is_dir() {
            self.parse_directory(path)?;
        } else if is_zip_file(path) {
            #[cfg(not(target_arch = "wasm32"))]
            self.parse_zip_file(path)?;
        } else {
            self.loaded.push(SupportedFormat::parse_from_path(path)?);
        }
        Ok(())
    }

    pub(crate) fn parse_raw_buffer(&mut self, buf: &[u8]) -> io::Result<()> {
        self.loaded.push(SupportedFormat::parse_from_buf(buf)?);
        Ok(())
    }

    fn parse_directory(&mut self, path: &Path) -> io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Err(e) = self.parse_directory(&path) {
                    log::warn!("{e}");
                }
            } else if is_zip_file(&path) {
                #[cfg(not(target_arch = "wasm32"))]
                self.parse_zip_file(&path)?;
            } else {
                match SupportedFormat::parse_from_path(&path) {
                    Ok(l) => self.loaded.push(l),
                    Err(e) => log::warn!("{e}"),
                }
            }
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_zip_file(&mut self, path: &Path) -> io::Result<()> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.is_file() {
                let mut contents = Vec::new();
                io::Read::read_to_end(&mut file, &mut contents)?;
                if let Ok(log) = SupportedFormat::parse_from_buf(&contents) {
                    self.loaded.push(log);
                }
            }
        }
        Ok(())
    }
}

fn is_zip_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}
