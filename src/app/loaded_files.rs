use plotinator_supported_formats::SupportedFormat;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self},
    path::Path,
};

/// Contains all supported logs in a single vector.
#[derive(Default, Deserialize, Serialize)]
pub struct LoadedFiles {
    pub(crate) loaded: Vec<SupportedFormat>,
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

        // Create a temporary directory for extracting HDF5 files
        let temp_dir = tempfile::tempdir()?;

        self.parse_zip_entries(&mut archive, "", 0, temp_dir.path())?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_zip_entries(
        &mut self,
        archive: &mut zip::ZipArchive<fs::File>,
        path_prefix: &str,
        depth: usize,
        temp_dir: &Path,
    ) -> io::Result<()> {
        use super::custom_files;
        use custom_files::CustomFileContent;

        const MAX_DEPTH: usize = 3;

        if depth > MAX_DEPTH {
            log::warn!("Reached recursion limit (max depth = {MAX_DEPTH}) for zip parsing");
            return Ok(());
        }

        // Get all file names in the current directory level
        let mut current_level_files = Vec::new();
        let mut subdirectories = std::collections::HashSet::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let file_name = file.name();

            // Skip files that don't start with the current path prefix
            if !file_name.starts_with(path_prefix) {
                continue;
            }

            // Get the relative path from the current prefix
            let relative_path = &file_name[path_prefix.len()..];

            // Skip empty relative paths (exact matches to prefix)
            if relative_path.is_empty() {
                continue;
            }

            // Check if this is a direct child or in a subdirectory
            let path_parts: Vec<&str> = relative_path.trim_start_matches('/').split('/').collect();

            if path_parts.len() == 1 && !relative_path.ends_with('/') {
                // This is a direct file in the current directory
                current_level_files.push(i);
            } else if path_parts.len() > 1 {
                // This is in a subdirectory, record the subdirectory name
                let subdir_name = path_parts[0];
                if !subdir_name.is_empty() {
                    let full_subdir_path = if path_prefix.is_empty() {
                        format!("{subdir_name}/")
                    } else {
                        format!("{path_prefix}{subdir_name}/")
                    };
                    subdirectories.insert(full_subdir_path);
                }
            }
        }

        // Process files in the current directory
        for file_index in current_level_files {
            let mut file = archive.by_index(file_index)?;

            if file.is_file() {
                let file_name = file.name().to_owned();
                let file_path = Path::new(&file_name);

                // Check if this is an HDF5 file
                if plotinator_hdf5::path_has_hdf5_extension(file_path) {
                    // Extract to temporary file and parse from path
                    let temp_file_name = file_path.file_name().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name")
                    })?;
                    let temp_file_path = temp_dir.join(temp_file_name);

                    // Extract the file
                    let mut temp_file = fs::File::create(&temp_file_path)?;
                    io::copy(&mut file, &mut temp_file)?;
                    temp_file.sync_all()?;
                    drop(temp_file); // Close the file before parsing

                    // Parse from the temporary file path
                    match SupportedFormat::parse_from_path(&temp_file_path) {
                        Ok(supported_format) => self.loaded.push(supported_format),
                        Err(e) => log::error!("Failed to parse HDF5 file {file_name}: {e}"),
                    }

                    // Clean up the temporary file
                    if let Err(e) = fs::remove_file(&temp_file_path) {
                        log::warn!("Failed to clean up temporary file {temp_file_path:?}: {e}");
                    }
                } else {
                    // Parse from buffer for non-HDF5 files
                    let mut contents = Vec::new();
                    io::Read::read_to_end(&mut file, &mut contents)?;

                    match super::custom_files::try_parse_custom_file_from_buf(&contents) {
                        Some(CustomFileContent::PlotUi(_)) => log::warn!(
                            "Ignoring custom Plot UI file found in Zip file, this would override all current loaded logs..."
                        ),
                        Some(CustomFileContent::PlotData(plotdata)) => self.loaded.extend(plotdata),
                        None => {
                            if let Ok(log) = SupportedFormat::parse_from_buf(&contents) {
                                self.loaded.push(log);
                            }
                        }
                    }
                }
            }
        }

        // Recursively process subdirectories
        for subdir_path in subdirectories {
            self.parse_zip_entries(archive, &subdir_path, depth + 1, temp_dir)?;
        }

        Ok(())
    }
}

fn is_zip_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}
