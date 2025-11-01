use plotinator_supported_formats::SupportedFormat;
use plotinator_ui_file_io::{ParseUpdate, UpdateChannel};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self},
    mem,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
};

/// Contains all supported logs in a single vector.
#[derive(Default, Deserialize, Serialize)]
pub struct LoadedFiles {
    pub(crate) loaded: Vec<SupportedFormat>,
}

impl LoadedFiles {
    /// Return a vector of immutable references to all logs
    pub fn loaded(&self) -> &[SupportedFormat] {
        &self.loaded
    }

    /// Take all the `loaded_files` currently stored and return them as a list
    pub fn take_loaded_files(&mut self) -> Vec<SupportedFormat> {
        mem::take(&mut self.loaded)
    }

    pub fn parse_path(&mut self, path: &Path, tx: UpdateChannel) -> anyhow::Result<()> {
        if path.is_dir() {
            self.parse_directory(path, tx)?;
        } else if is_zip_file(path) {
            #[cfg(not(target_arch = "wasm32"))]
            self.parse_zip_file(path, tx)?;
        } else {
            self.loaded
                .push(SupportedFormat::parse_from_path(path, tx)?);
        }
        Ok(())
    }

    pub fn parse_raw_buffer(&mut self, buf: &[u8]) -> io::Result<()> {
        self.loaded.push(SupportedFormat::parse_from_buf(buf)?);
        Ok(())
    }

    fn parse_directory(&mut self, path: &Path, tx: UpdateChannel) -> io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Err(e) = self.parse_directory(&path, tx.clone()) {
                    log::warn!("{e}");
                }
            } else if is_zip_file(&path) {
                #[cfg(not(target_arch = "wasm32"))]
                self.parse_zip_file(&path, tx.clone())?;
            } else {
                match SupportedFormat::parse_from_path(&path, tx.clone()) {
                    Ok(l) => self.loaded.push(l),
                    Err(e) => log::warn!("{e}"),
                }
            }
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_zip_file(&mut self, path: &Path, tx: UpdateChannel) -> io::Result<()> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        // Create a temporary directory for extracting HDF5 files
        let temp_dir = tempfile::tempdir()?;

        self.parse_zip_entries(&mut archive, "", 0, temp_dir.path(), tx)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_zip_entries(
        &mut self,
        archive: &mut zip::ZipArchive<fs::File>,
        path_prefix: &str,
        depth: usize,
        temp_dir: &Path,
        tx: UpdateChannel,
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
                    match SupportedFormat::parse_from_path(&temp_file_path, tx.clone()) {
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
            self.parse_zip_entries(archive, &subdir_path, depth + 1, temp_dir, tx.clone())?;
        }

        Ok(())
    }
}

fn is_zip_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use plotinator_log_if::plotable::Plotable as _;
    use plotinator_test_util::test_file_defs::frame_altimeters::*;
    use plotinator_test_util::test_file_defs::mbed_motor_control::*;
    use std::io::Write as _;
    use std::sync::mpsc::channel;
    use tempfile::TempDir;
    use testresult::TestResult;
    use zip::write::ExtendedFileOptions;
    use zip::{ZipWriter, write::FileOptions};

    #[test]
    fn test_parse_zip_file_with_nested_directories() -> TestResult {
        // Create a temporary zip file with nested structure
        let temp_dir = TempDir::new()?;
        let zip_path = temp_dir.path().join("test_archive.zip");
        let zip_file = std::fs::File::create(&zip_path)?;
        let mut zip_writer = ZipWriter::new(zip_file);

        let options: FileOptions<'_, ExtendedFileOptions> =
            FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // Level 1: Root files
        zip_writer.start_file("root_file.pid", options.clone())?;
        zip_writer.write_all(MBED_PID_V1_BYTES)?;

        // Level 2: First subdirectory
        zip_writer.start_file("level1/status_log.status", options.clone())?;
        zip_writer.write_all(MBED_STATUS_V1_BYTES)?;

        // Level 3: Second subdirectory with HDF5 file
        zip_writer.start_file("level1/level2/altimeters.h5", options.clone())?;
        zip_writer.write_all(FRAME_ALTIMETERS_BYTES)?;

        // Level 4: Third subdirectory (should still be parsed, at max depth)
        zip_writer.start_file("level1/level2/level3/another_pid.bin", options.clone())?;
        zip_writer.write_all(MBED_PID_V1_BYTES)?;

        // Level 5: Fourth subdirectory (should be ignored, beyond max depth)
        zip_writer.start_file("level1/level2/level3/level4/ignored.pid", options.clone())?;
        zip_writer.write_all(MBED_PID_V1_BYTES)?;

        // Additional nested structure to test directory traversal
        zip_writer.start_file("another_branch/data.status", options.clone())?;
        zip_writer.write_all(MBED_STATUS_V1_BYTES)?;

        zip_writer.start_file("another_branch/deep/nested/file.pid", options)?;
        zip_writer.write_all(MBED_PID_V1_BYTES)?;

        zip_writer.finish()?;

        // Test parsing the zip file
        let mut loaded_files = LoadedFiles::default();
        let (tx, _rx) = channel();
        let update_chan = UpdateChannel::new(tx);
        loaded_files.parse_zip_file(&zip_path, update_chan)?;

        let loaded = loaded_files.loaded();

        // Verify we got the expected number of files
        // Should parse 6 files total:
        // - root_file.pid (level 1)
        // - level1/status_log.status (level 2)
        // - level1/level2/altimeters.h5 (level 3)
        // - level1/level2/level3/another_pid.pid (level 3)
        // - another_branch/data.status (level 2)
        // - another_branch/deep/nested/file.pid (level 3)
        // Should NOT parse level4/ignored.pid (level 4, beyond max depth)
        assert_eq!(
            loaded.len(),
            6,
            "Expected 6 files to be parsed from zip archive, got: {loaded:?}"
        );

        // Verify we have different types of files
        let mut pid_count = 0;
        let mut status_count = 0;
        let mut hdf5_count = 0;

        for format in loaded {
            let name = format.descriptive_name();
            eprintln!("{name}");
            match name {
                "Mbed PID v1" => pid_count += 1,
                "Mbed Status v1" => status_count += 1,
                "Frame altimeters" => hdf5_count += 1,
                _ => panic!("unexpected name: {name}"),
            };
        }

        // Verify we parsed the expected file types
        assert_eq!(pid_count, 3, "Expected 3 PID files");
        assert_eq!(status_count, 2, "Expected 2 Status files");
        assert_eq!(hdf5_count, 1, "Expected 1 HDF5 file");

        Ok(())
    }
}
