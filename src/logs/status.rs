use super::{parse_to_vec, LogEntry};
use crate::util::{parse_timestamp, read_f32, read_u32};
use serde_big_array::BigArray;
use std::{io, mem};

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLog {
    header: StatusLogHeader,
    entries: Vec<StatusLogEntry>,
    timestamps_with_state_changes: Vec<(u32, u8)>, // for memoization
}

fn parse_timestamps_with_state_changes(entries: &[StatusLogEntry]) -> Vec<(u32, u8)> {
    let mut result = Vec::new();
    let mut last_state = None;

    for entry in entries.iter() {
        // Check if the current state is different from the last recorded state
        if last_state != Some(entry.motor_state) {
            result.push((entry.timestamp_ms, entry.motor_state));
            last_state = Some(entry.motor_state);
        }
    }
    result
}

impl StatusLog {
    pub fn from_buf(bytes: &mut &[u8]) -> io::Result<Self> {
        let mut pos = 0;
        let header = StatusLogHeader::from_buf(bytes)?;
        pos += StatusLogHeader::packed_footprint();
        let vec_of_entries = parse_to_vec::<StatusLogEntry>(&mut &bytes[pos..]);
        let timestamps_with_state_changes = parse_timestamps_with_state_changes(&vec_of_entries);
        Ok(Self {
            header,
            entries: vec_of_entries,
            timestamps_with_state_changes,
        })
    }

    pub fn entries(&self) -> &[StatusLogEntry] {
        &self.entries
    }

    pub fn timestamps_with_state_changes(&self) -> &[(u32, u8)] {
        &self.timestamps_with_state_changes
    }
}

impl std::fmt::Display for StatusLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Header: {}", self.header)?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}
#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLogHeader {
    #[serde(with = "BigArray")]
    unique_description: [u8; 128],
    version: u16,
}

impl StatusLogHeader {
    pub const UNIQUE_DESCRIPTION: &'static str = "MBED-MOTOR-CONTROL-STATUS-LOG";
    fn unique_description(&self) -> String {
        let uniq_desc = super::parse_unique_description(self.unique_description);
        uniq_desc
    }

    pub fn is_buf_header(bytes: &mut &[u8]) -> io::Result<bool> {
        let deserialized = Self::from_buf(bytes)?;
        let is_header = deserialized.unique_description() == Self::UNIQUE_DESCRIPTION;
        Ok(is_header)
    }
}

impl StatusLogHeader {
    fn from_buf(bytes: &mut &[u8]) -> io::Result<Self> {
        let mut unique_description = [0; 128];
        unique_description.clone_from_slice(&bytes[..128]);
        let version = u16::from_le_bytes([bytes[128], bytes[129]]);
        Ok(Self {
            unique_description,
            version,
        })
    }

    fn packed_footprint() -> usize {
        128 * mem::size_of::<u8>() // unique description
        +
        mem::size_of::<u16>() // Version
    }
}

impl std::fmt::Display for StatusLogHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-v{}", self.unique_description(), self.version)
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLogEntry {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub engine_temp: f32,
    pub fan_on: bool,
    pub vbat: f32,
    pub setpoint: f32,
    pub motor_state: u8,
}

impl std::fmt::Display for StatusLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} {} {} {} {}",
            self.timestamp_ms,
            self.engine_temp,
            self.fan_on,
            self.vbat,
            self.setpoint,
            self.motor_state
        )
    }
}

impl LogEntry for StatusLogEntry {
    fn from_buf(bytes: &mut &[u8]) -> io::Result<Self> {
        let timestamp_ms = read_u32(bytes)?;
        let timestamp_ms_str = parse_timestamp(timestamp_ms);
        Ok(Self {
            timestamp_ms_str,
            timestamp_ms,
            engine_temp: read_f32(bytes)?,
            fan_on: bytes[0] == 1,
            vbat: read_f32(&mut &bytes[1..])?,
            setpoint: read_f32(&mut &bytes[5..])?,
            motor_state: bytes[9],
        })
    }

    fn packed_footprint() -> usize {
        mem::size_of::<u32>() // timestamp
            + mem::size_of::<f32>() // engine temp
            + mem::size_of::<u8>() // fan_on 
            + mem::size_of::<f32>() // vbat
            + mem::size_of::<f32>() // setpoint
            + mem::size_of::<u8>() // motor state
    }

    fn timestamp_ms(&self) -> u32 {
        self.timestamp_ms
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_deserialize() {
        let data = fs::read("test_data/status_20240906_081236_00.bin").unwrap();
        let status_log = StatusLog::from_buf(&mut data.as_slice()).unwrap();
        eprintln!("{}", status_log.header);
        assert_eq!(
            status_log.header.unique_description(),
            StatusLogHeader::UNIQUE_DESCRIPTION
        );
        assert_eq!(status_log.header.version, 0);
        let first_entry = status_log.entries.first().unwrap();
        assert_eq!(first_entry.engine_temp, 0.0);
        assert_eq!(first_entry.fan_on, true);
        assert_eq!(first_entry.vbat, 2.0);
        assert_eq!(first_entry.setpoint, 3.0);
        assert_eq!(first_entry.motor_state, 4);
        let second_entry = status_log.entries.get(1).unwrap();
        assert_eq!(second_entry.engine_temp, 123.4);
        assert_eq!(second_entry.fan_on, false);
        assert_eq!(second_entry.vbat, 2.34);
        assert_eq!(second_entry.setpoint, 3.45);
        assert_eq!(second_entry.motor_state, 5);
        //eprintln!("{status_log}");
    }
}
