use egui::DroppedFile;
use log_if::prelude::*;
use serde::{Deserialize, Serialize};
use skytem_logs::{
    generator::{GeneratorLog, GeneratorLogEntry},
    mbed_motor_control::{pid::pidlog::PidLog, status::statuslog::StatusLog},
};
use std::{
    fs,
    io::{self, BufReader},
    path::Path,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct ParsedBytes(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct TotalBytes(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct ParseInfo {
    parsed_bytes: ParsedBytes,
    total_bytes: TotalBytes,
}

impl ParseInfo {
    pub fn new(parsed_bytes: ParsedBytes, total_bytes: TotalBytes) -> Self {
        let parsed = parsed_bytes.0;
        let total = total_bytes.0;

        debug_assert!(
            parsed <= total,
            "Unsound condition, parsed more than the total bytes! Parsed: {parsed}, total: {total}"
        );
        Self {
            parsed_bytes,
            total_bytes,
        }
    }
    pub fn parsed_bytes(&self) -> usize {
        self.parsed_bytes.0
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes.0
    }

    pub fn remainder_bytes(&self) -> usize {
        self.total_bytes.0 - self.parsed_bytes.0
    }
}

/// Represents a supported log, which can be any of the supported log types.
///
/// This simply serves to encapsulate all the supported logs in a single type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SupportedLog {
    MbedPid(PidLog, ParseInfo),
    MbedStatus(StatusLog, ParseInfo),
    Generator(GeneratorLog, ParseInfo),
}

impl SupportedLog {
    /// Attempts to parse a log from raw content.
    fn parse_from_content(mut content: &[u8]) -> io::Result<Self> {
        let total_bytes = content.len();
        log::debug!("Parsing content of length: {total_bytes}");
        let log = if PidLog::is_buf_valid(content) {
            let (log, read_bytes) = PidLog::from_reader(&mut content)?;
            log::debug!("Read: {read_bytes} bytes");
            Self::MbedPid(
                log,
                ParseInfo::new(ParsedBytes(read_bytes), TotalBytes(total_bytes)),
            )
        } else if StatusLog::is_buf_valid(content) {
            let (log, read_bytes) = StatusLog::from_reader(&mut content)?;
            Self::MbedStatus(
                log,
                ParseInfo::new(ParsedBytes(read_bytes), TotalBytes(read_bytes)),
            )
        } else if GeneratorLogEntry::is_bytes_valid_generator_log_entry(content) {
            let (log, read_bytes) = GeneratorLog::from_reader(&mut content)?;
            Self::Generator(
                log,
                ParseInfo::new(ParsedBytes(read_bytes), TotalBytes(total_bytes)),
            )
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unrecognized log type",
            ));
        };
        log::debug!("Got: {}", log.descriptive_name());
        Ok(log)
    }

    /// Attempts to parse a log from a file path.
    fn parse_from_path(path: &Path) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        let total_bytes = file.metadata()?.len() as usize;
        log::debug!("Parsing content of length: {total_bytes}");
        let mut reader = BufReader::new(file);

        let log = if PidLog::file_is_valid(path) {
            let (log, parsed_bytes) = PidLog::from_reader(&mut reader)?;
            log::debug!("Read: {parsed_bytes} bytes");
            Self::MbedPid(
                log,
                ParseInfo::new(ParsedBytes(parsed_bytes), TotalBytes(total_bytes)),
            )
        } else if StatusLog::file_is_valid(path) {
            let (log, parsed_bytes) = StatusLog::from_reader(&mut reader)?;
            Self::MbedStatus(
                log,
                ParseInfo::new(ParsedBytes(parsed_bytes), TotalBytes(total_bytes)),
            )
        } else if GeneratorLog::file_is_generator_log(path).unwrap_or(false) {
            let (log, parsed_bytes) = GeneratorLog::from_reader(&mut reader)?;
            Self::Generator(
                log,
                ParseInfo::new(ParsedBytes(parsed_bytes), TotalBytes(total_bytes)),
            )
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unrecognized log type",
            ));
        };
        log::debug!("Got: {}", log.descriptive_name());
        Ok(log)
    }

    pub fn parse_info(&self) -> ParseInfo {
        match self {
            Self::MbedPid(_, parse_info)
            | Self::MbedStatus(_, parse_info)
            | Self::Generator(_, parse_info) => *parse_info,
        }
    }
}

impl Plotable for SupportedLog {
    fn raw_plots(&self) -> &[RawPlot] {
        match self {
            Self::MbedPid(l, _) => l.raw_plots(),
            Self::MbedStatus(l, _) => l.raw_plots(),
            Self::Generator(l, _) => l.raw_plots(),
        }
    }

    fn first_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            Self::MbedPid(l, _) => l.first_timestamp(),
            Self::MbedStatus(l, _) => l.first_timestamp(),
            Self::Generator(l, _) => l.first_timestamp(),
        }
    }

    fn descriptive_name(&self) -> &str {
        match self {
            Self::MbedPid(l, _) => l.descriptive_name(),
            Self::MbedStatus(l, _) => l.descriptive_name(),
            Self::Generator(l, _) => l.descriptive_name(),
        }
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        match self {
            Self::MbedPid(l, _) => l.labels(),
            Self::MbedStatus(l, _) => l.labels(),
            Self::Generator(l, _) => l.labels(),
        }
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        match self {
            Self::MbedPid(l, _) => l.metadata(),
            Self::MbedStatus(l, _) => l.metadata(),
            Self::Generator(l, _) => l.metadata(),
        }
    }
    // Implement any methods required by the Plotable trait for SupportedLog
}

/// Contains all supported logs in a single vector.
#[derive(Default, Deserialize, Serialize)]
pub struct SupportedLogs {
    logs: Vec<SupportedLog>,
}

impl SupportedLogs {
    /// Return a vector of immutable references to all logs
    pub fn logs(&self) -> &[SupportedLog] {
        &self.logs
    }

    /// Take all the logs currently stored in [`SupportedLogs`] and return them as a list
    pub fn take_logs(&mut self) -> Vec<SupportedLog> {
        self.logs.drain(..).collect()
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
            self.logs.push(SupportedLog::parse_from_content(content)?);
        } else if let Some(path) = &file.path {
            if path.is_dir() {
                self.parse_directory(path)?;
            } else if is_zip_file(path) {
                #[cfg(not(target_arch = "wasm32"))]
                self.parse_zip_file(path)?;
            } else {
                self.logs.push(SupportedLog::parse_from_path(path)?);
            }
        }
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
                match SupportedLog::parse_from_path(&path) {
                    Ok(l) => self.logs.push(l),
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
                if let Ok(log) = SupportedLog::parse_from_content(&contents) {
                    self.logs.push(log);
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

#[cfg(test)]
mod tests {
    use super::*;
    const TEST_DATA_STATUS: &str =
        "test_data/mbed_motor_control/v1/20240926_121708/status_20240926_121708_00.bin";

    const TEST_DATA_PID: &str =
        "test_data/mbed_motor_control/v1/20240926_121708/pid_20240926_121708_00.bin";

    #[test]
    fn test_supported_logs_dyn_vec() {
        let data = fs::read(TEST_DATA_STATUS).unwrap();
        let (status_log, _status_log_bytes_read) =
            StatusLog::from_reader(&mut data.as_slice()).unwrap();

        let data = fs::read(TEST_DATA_PID).unwrap();
        let (pidlog, _pid_log_bytes_read) = PidLog::from_reader(&mut data.as_slice()).unwrap();

        let v: Vec<Box<dyn Plotable>> = vec![Box::new(status_log), Box::new(pidlog)];
        assert_eq!(v.len(), 2);
    }
}
