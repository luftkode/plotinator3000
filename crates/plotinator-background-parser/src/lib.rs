use plotinator_supported_formats::SupportedFormat;
use plotinator_ui_file_io::{ParseUpdate, UpdateChannel};
use smallvec::{SmallVec, smallvec};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
};
use tempfile::TempDir;

use crate::loaded_format::LoadedSupportedFormat;

pub mod loaded_format;

pub struct ParserThreads {
    update_tx: UpdateChannel,
    threads: Vec<ParserThread>,
    tmp_dir: TempDir,
}

impl ParserThreads {
    pub fn new(tx: Sender<ParseUpdate>) -> Self {
        Self {
            update_tx: UpdateChannel::new(tx),
            threads: vec![],
            tmp_dir: tempfile::tempdir().expect(
                "Failed creating temporary directory for parsing HDF5 files from zip archives",
            ),
        }
    }

    pub fn parse_path(&mut self, path: &Path) -> io::Result<()> {
        if is_zip_file(path) {
            self.parse_zip_file(path)?;
        } else if path.is_dir() {
            self.parse_directory(path)?;
        } else {
            debug_assert!(path.is_file());
            let new_thread = ParserThread::new(path.to_owned(), self.update_tx.clone());
            self.threads.push(new_thread);
        }
        Ok(())
    }

    fn parse_directory(&mut self, path: &Path) -> io::Result<()> {
        debug_assert!(path.is_dir());
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Err(e) = self.parse_directory(&path) {
                    log::warn!("{e}");
                }
            } else {
                self.parse_path(&path)?;
            }
        }
        Ok(())
    }

    fn parse_zip_file(&mut self, path: &Path) -> io::Result<()> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let tmp_dir = self.tmp_dir.path().to_owned();
        self.parse_zip_entries(&mut archive, "", 0, &tmp_dir, &self.update_tx.clone())?;
        Ok(())
    }

    fn parse_zip_entries(
        &mut self,
        archive: &mut zip::ZipArchive<fs::File>,
        path_prefix: &str,
        depth: usize,
        temp_dir: &Path,
        _tx: &UpdateChannel,
    ) -> io::Result<()> {
        const MAX_DEPTH: usize = 5;

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

                self.parse_path(&temp_file_path)?;
            }
        }

        // Recursively process subdirectories
        for subdir_path in subdirectories {
            self.parse_zip_entries(archive, &subdir_path, depth + 1, temp_dir, _tx)?;
        }

        Ok(())
    }

    pub fn poll(&mut self) -> SmallVec<[LoadedSupportedFormat; 1]> {
        let mut loaded_formats = smallvec![];
        let running_threads: Vec<_> = self.threads.drain(..).collect();
        for t in running_threads {
            if t.is_finished() {
                if let Some(lf) = t.finish() {
                    loaded_formats.push(lf);
                }
            } else {
                self.threads.push(t);
            }
        }
        loaded_formats
    }

    pub fn active_threads(&self) -> bool {
        !self.threads.is_empty()
    }
}

pub struct ParserThread {
    path: PathBuf,
    update_tx: UpdateChannel,
    handle: Option<thread::JoinHandle<anyhow::Result<LoadedSupportedFormat>>>,
}

impl ParserThread {
    pub fn new(path: PathBuf, update_tx: UpdateChannel) -> Self {
        let thread_name = path.to_string_lossy().into_owned();
        log::debug!("Starting parser thread: {thread_name}");
        let handle = thread::Builder::new()
            .name(thread_name)
            .spawn({
                let p = path.clone();
                let update_tx = update_tx.clone();
                move || {
                    let parsed_format = SupportedFormat::parse_from_path(&p, update_tx.clone())?;
                    update_tx.send(ParseUpdate::Progress {
                        path: p.clone(),
                        progress: 0.8,
                    });
                    let mut loaded_format = LoadedSupportedFormat::new(parsed_format);
                    loaded_format.cook_all();
                    update_tx.send(ParseUpdate::Progress {
                        path: p.clone(),
                        progress: 0.99,
                    });

                    Ok(loaded_format)
                }
            })
            .expect("Failed spawning parser thread");
        Self {
            path,
            update_tx,
            handle: Some(handle),
        }
    }

    pub fn is_finished(&self) -> bool {
        debug_assert!(
            self.handle.is_some(),
            "called is_finished on a parser thread that should've been finished/consumed"
        );
        self.handle.as_ref().is_some_and(|h| h.is_finished())
    }

    pub fn finish(mut self) -> Option<LoadedSupportedFormat> {
        let h = self
            .handle
            .take()
            .expect("tried finishing a thread that was already finished");

        match h.join() {
            Ok(parse_res) => match parse_res {
                Ok(s) => {
                    self.update_tx.send(ParseUpdate::Completed {
                        path: self.path.clone(),
                        final_format: s.format_name().to_owned(),
                    });
                    return Some(s);
                }
                Err(e) => self.update_tx.send(ParseUpdate::Failed {
                    path: self.path.clone(),
                    error_msg: format!("Not valid: {e}"),
                }),
            },
            Err(e) => {
                self.update_tx.send(ParseUpdate::Failed {
                    path: self.path.clone(),
                    error_msg: format!(
                        "Unexpected crash: {e:?}, please file an issue on the github page"
                    ),
                });
            }
        }
        None
    }
}

fn is_zip_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

#[cfg(test)]
mod tests {
    use super::*;
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

        // Level 7: Fourth subdirectory (should be ignored, beyond max depth)
        zip_writer.start_file(
            "level1/level2/level3/level4/level5/level6/level7/ignored.pid",
            options.clone(),
        )?;
        zip_writer.write_all(MBED_PID_V1_BYTES)?;

        // Additional nested structure to test directory traversal
        zip_writer.start_file("another_branch/data.status", options.clone())?;
        zip_writer.write_all(MBED_STATUS_V1_BYTES)?;

        zip_writer.start_file("another_branch/deep/nested/file.pid", options)?;
        zip_writer.write_all(MBED_PID_V1_BYTES)?;

        zip_writer.finish()?;

        // Test parsing the zip file
        let (tx, _rx) = channel();
        let mut parser_threads = ParserThreads::new(tx);
        parser_threads.parse_path(&zip_path)?;

        let mut loaded = vec![];
        while parser_threads.active_threads() {
            for loaded_format in parser_threads.poll() {
                loaded.push(loaded_format);
            }
        }

        // Verify we got the expected number of files
        // Should parse 6 files total:
        // - root_file.pid (level 1)
        // - level1/status_log.status (level 2)
        // - level1/level2/altimeters.h5 (level 3)
        // - level1/level2/level3/another_pid.pid (level 3)
        // - another_branch/data.status (level 2)
        // - another_branch/deep/nested/file.pid (level 3)
        // Should NOT parse level4/ignored.pid (level 7, beyond max depth)
        assert_eq!(
            loaded.len(),
            6,
            "Expected 6 files to be parsed from zip archive"
        );

        // Verify we have different types of files
        let mut pid_count = 0;
        let mut status_count = 0;
        let mut hdf5_count = 0;

        for format in loaded {
            let name = format.format_name();
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
