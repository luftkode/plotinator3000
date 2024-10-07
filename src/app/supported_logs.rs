use egui::DroppedFile;
use log_if::prelude::*;
use serde::{Deserialize, Serialize};
use skytem_logs::{
    generator::{GeneratorLog, GeneratorLogEntry},
    mbed_motor_control::{
        pid::{header::PidLogHeader, PidLog},
        status::{header::StatusLogHeader, StatusLog},
        MbedMotorControlLogHeader,
    },
};
use std::{
    fs,
    io::{self, BufReader},
    path::{self, Path},
};

/// In the ideal future, this explicit list of supported logs is instead just a vector of log interfaces (traits)
/// that would require the log interface to also support a common way for plotting logs
#[derive(Default, Deserialize, Serialize)]
pub struct SupportedLogs {
    pid_log: Vec<PidLog>,
    status_log: Vec<StatusLog>,
    generator_log: Vec<GeneratorLog>,
}

impl SupportedLogs {
    /// Return a vector of immutable references to all logs
    pub fn logs(&self) -> Vec<&dyn Plotable> {
        let mut all_logs: Vec<&dyn Plotable> = Vec::new();
        for pl in &self.pid_log {
            all_logs.push(pl);
        }
        for sl in &self.status_log {
            all_logs.push(sl);
        }
        for gl in &self.generator_log {
            all_logs.push(gl);
        }
        all_logs
    }

    /// Take all the logs currently store in [`SupportedLogs`] and return them as a list
    pub fn take_logs(&mut self) -> Vec<Box<dyn Plotable>> {
        let mut all_logs: Vec<Box<dyn Plotable>> = Vec::new();
        all_logs.extend(self.pid_log.drain(..).map(|log| log.into()));
        all_logs.extend(self.status_log.drain(..).map(|log| log.into()));
        all_logs.extend(self.generator_log.drain(..).map(|log| log.into()));

        all_logs
    }

    /// Parse dropped files to supported logs.
    ///
    /// ### Note to developers who are not seasoned Rust devs :)
    /// This cannot take `&mut self` as that breaks ownership rules when looping over dropped files
    /// meaning you would be forced to make a copy which isn't actually needed, but required for it to compile.
    pub fn parse_dropped_files(&mut self, dropped_files: &[DroppedFile]) -> io::Result<()> {
        for file in dropped_files {
            log::debug!("Parsing dropped file: {file:?}");
            self.parse_file(file)?;
        }
        Ok(())
    }

    fn parse_file(&mut self, file: &DroppedFile) -> io::Result<()> {
        if let Some(content) = file.bytes.as_ref() {
            // This is how content is made accessible via drag-n-drop in a browser
            self.parse_content(content)?;
        } else if let Some(path) = &file.path {
            // This is how content is accessible via drag-n-drop when the app is running natively
            log::debug!("path: {path:?}");
            if path.is_dir() {
                self.parse_directory(path)?;
            } else if is_zip_file(path) {
                #[cfg(not(target_arch = "wasm32"))]
                self.parse_zip_file(path)?;
            } else {
                self.parse_path(path)?;
            }
        } else {
            unreachable!("What is this content??")
        }
        Ok(())
    }

    // Parsing directory on native
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
            } else if let Err(e) = self.parse_path(&path) {
                log::warn!("{e}");
            }
        }
        Ok(())
    }

    // Parsing dropped content on web
    fn parse_content(&mut self, mut content: &[u8]) -> io::Result<()> {
        if PidLogHeader::is_buf_header(content).unwrap_or(false) {
            self.pid_log.push(PidLog::from_reader(&mut content)?);
        } else if StatusLogHeader::is_buf_header(content).unwrap_or(false) {
            self.status_log.push(StatusLog::from_reader(&mut content)?);
        } else if GeneratorLogEntry::is_bytes_valid_generator_log_entry(content) {
            self.generator_log
                .push(GeneratorLog::from_reader(&mut content)?);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unrecognized file",
            ));
        }
        Ok(())
    }

    // Parse file on native
    fn parse_path(&mut self, path: &path::Path) -> io::Result<()> {
        if PidLogHeader::file_starts_with_header(path).unwrap_or(false) {
            let f = fs::File::open(path)?;
            self.pid_log
                .push(PidLog::from_reader(&mut BufReader::new(f))?);
        } else if StatusLogHeader::file_starts_with_header(path).unwrap_or(false) {
            let f = fs::File::open(path)?;
            self.status_log
                .push(StatusLog::from_reader(&mut BufReader::new(f))?);
        } else if GeneratorLog::file_is_generator_log(path).unwrap_or(false) {
            let f = fs::File::open(path)?;
            self.generator_log
                .push(GeneratorLog::from_reader(&mut BufReader::new(f))?);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unrecognized file: {}", path.to_string_lossy()),
            ));
        }
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn parse_zip_file(&mut self, path: &Path) -> io::Result<()> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            log::debug!("Parsing zipped: {}", file.name());

            if file.is_dir() {
                continue;
            }

            let mut contents = Vec::new();
            io::Read::read_to_end(&mut file, &mut contents)?;

            if let Err(e) = self.parse_content(&contents) {
                log::warn!("Failed to parse file {} in zip: {}", file.name(), e);
            }
        }
        Ok(())
    }
}

fn is_zip_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_DATA_STATUS: &str =
        "test_data/mbed_motor_control/20240926_121708/status_20240926_121708_00.bin";

    const TEST_DATA_PID: &str =
        "test_data/mbed_motor_control/20240926_121708/pid_20240926_121708_00.bin";

    #[test]
    fn test_supported_logs_dyn_vec() {
        let data = fs::read(TEST_DATA_STATUS).unwrap();
        let status_log = StatusLog::from_reader(&mut data.as_slice()).unwrap();

        let data = fs::read(TEST_DATA_PID).unwrap();
        let pidlog = PidLog::from_reader(&mut data.as_slice()).unwrap();

        let v: Vec<Box<dyn Plotable>> = vec![Box::new(status_log), Box::new(pidlog)];
        assert_eq!(v.len(), 2);
    }
}
