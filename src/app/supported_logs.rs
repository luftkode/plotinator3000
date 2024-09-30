use egui::DroppedFile;
use log_if::{Log, Plotable};
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
    path,
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
    pub fn logs(&mut self) -> Vec<Box<dyn Plotable>> {
        let mut all_logs: Vec<Box<dyn Plotable>> = Vec::new();
        all_logs.extend(self.pid_log.drain(..).map(|log| log.into()));
        all_logs.extend(self.status_log.drain(..).map(|log| log.into()));
        all_logs.extend(self.generator_log.drain(..).map(|log| log.into()));

        all_logs
    }

    /// Parse dropped files to supported logs. Only parses and stores log types that haven't already been parsed successfully
    ///
    /// ### Note to developers who are not seasoned Rust devs :)
    /// This cannot take `&mut self` as that breaks ownership rules when looping over dropped files
    /// meaning you would be forced to make a copy which isn't actually needed, but required for it to compile.
    pub fn parse_dropped_files(dropped_files: &[DroppedFile], logs: &mut Self) -> io::Result<()> {
        for file in dropped_files {
            parse_file(file, logs)?;
        }
        Ok(())
    }
}

fn parse_file(file: &DroppedFile, logs: &mut SupportedLogs) -> io::Result<()> {
    if let Some(content) = file.bytes.as_ref().map(|b| b.as_ref()) {
        // This is how content is made accessible via drag-n-drop in a browser
        parse_content(content, logs)?;
    } else if let Some(path) = &file.path {
        // This is how content is accessible via drag-n-drop when the app is running natively
        parse_path(path, logs)?;
    } else {
        unreachable!("What is this content??")
    }
    Ok(())
}

fn parse_content(mut content: &[u8], logs: &mut SupportedLogs) -> io::Result<()> {
    if PidLogHeader::is_buf_header(content).unwrap_or(false) {
        logs.pid_log.push(PidLog::from_reader(&mut content)?);
    } else if StatusLogHeader::is_buf_header(content).unwrap_or(false) {
        logs.status_log.push(StatusLog::from_reader(&mut content)?);
    } else if GeneratorLogEntry::is_bytes_valid_generator_log_entry(content) {
        logs.generator_log
            .push(GeneratorLog::from_reader(&mut content)?);
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unrecognized file",
        ));
    }
    Ok(())
}

fn parse_path(path: &path::Path, logs: &mut SupportedLogs) -> io::Result<()> {
    if PidLogHeader::file_starts_with_header(path).unwrap_or(false) {
        let f = fs::File::open(path)?;
        logs.pid_log
            .push(PidLog::from_reader(&mut BufReader::new(f))?);
    } else if StatusLogHeader::file_starts_with_header(path).unwrap_or(false) {
        let f = fs::File::open(path)?;
        logs.status_log
            .push(StatusLog::from_reader(&mut BufReader::new(f))?);
    } else if GeneratorLog::file_is_generator_log(path).unwrap_or(false) {
        let f = fs::File::open(path)?;
        logs.generator_log
            .push(GeneratorLog::from_reader(&mut BufReader::new(f))?);
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unrecognized file: {}", path.to_string_lossy()),
        ));
    }
    Ok(())
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
