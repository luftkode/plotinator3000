use crate::logs::{
    generator::{GeneratorLog, GeneratorLogEntry},
    mbed_motor_control::{
        pid::{PidLog, PidLogHeader},
        status::{StatusLog, StatusLogHeader},
        MbedMotorControlLogHeader,
    },
    Log,
};
use egui::DroppedFile;
use std::{fs, io::BufReader};

/// In the ideal future, this explicit list of supported logs is instead just a vector of log interfaces (traits)
/// that would require the log interface to also support a common way for plotting logs
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct SupportedLogs {
    pid_log: Option<PidLog>,
    status_log: Option<StatusLog>,
    generator_log: Option<GeneratorLog>,
}

impl SupportedLogs {
    pub fn mbed_pid_log(&self) -> Option<&PidLog> {
        self.pid_log.as_ref()
    }
    pub fn mbed_status_log(&self) -> Option<&StatusLog> {
        self.status_log.as_ref()
    }
    pub fn generator_log(&self) -> Option<&GeneratorLog> {
        self.generator_log.as_ref()
    }

    /// Parse dropped files to supported logs. Only parses and stores log types that haven't already been parsed succesfully
    ///
    /// ### Note to developers who are not seasoned Rust devs :)
    /// This cannot take `&mut self` as that breaks ownership rules when looping over dropped files
    /// meaning you would be forced to make a copy which isn't actually needed, but required for it to compile.
    pub fn parse_dropped_files(dropped_files: &[DroppedFile], logs: &mut SupportedLogs) {
        for file in dropped_files {
            parse_file(&file, logs);
        }
    }
}

fn parse_file(file: &DroppedFile, logs: &mut SupportedLogs) {
    if let Some(content) = file.bytes.as_ref().map(|b| b.as_ref()) {
        // This is how content is made accesible via drag-n-drop in a browser
        parse_content(content, logs);
    } else if let Some(path) = &file.path {
        // This is how content is accesible via drag-n-drop when the app is running natively
        parse_path(path, logs);
    }
}

fn parse_content(mut content: &[u8], logs: &mut SupportedLogs) {
    if logs.pid_log.is_none() && PidLogHeader::is_buf_header(content).unwrap_or(false) {
        logs.pid_log = PidLog::from_reader(&mut content).ok();
    } else if logs.status_log.is_none() && StatusLogHeader::is_buf_header(content).unwrap_or(false)
    {
        logs.status_log = StatusLog::from_reader(&mut content).ok();
    } else if logs.generator_log.is_none()
        && GeneratorLogEntry::is_bytes_valid_generator_log_entry(content)
    {
        logs.generator_log = GeneratorLog::from_reader(&mut content).ok();
    }
}

fn parse_path(path: &std::path::Path, logs: &mut SupportedLogs) {
    if logs.pid_log.is_none() && PidLogHeader::file_starts_with_header(path).unwrap_or(false) {
        logs.pid_log = fs::File::open(path)
            .ok()
            .and_then(|file| PidLog::from_reader(&mut BufReader::new(file)).ok());
    } else if logs.status_log.is_none()
        && StatusLogHeader::file_starts_with_header(path).unwrap_or(false)
    {
        logs.status_log = fs::File::open(path)
            .ok()
            .and_then(|file| StatusLog::from_reader(&mut BufReader::new(file)).ok());
    } else if logs.generator_log.is_none()
        && GeneratorLog::file_is_generator_log(path).unwrap_or(false)
    {
        logs.generator_log = fs::File::open(path)
            .ok()
            .and_then(|file| GeneratorLog::from_reader(&mut BufReader::new(file)).ok());
    }
}
