use serde_big_array::BigArray;
use std::{io, mem};

use super::{parse_to_vec, LogEntry};
use crate::util::{parse_timestamp, read_f32, read_u32};

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLog {
    header: PidLogHeader,
    entries: Vec<PidLogEntry>,
}

impl PidLog {
    pub fn from_buf(bytes: &mut &[u8]) -> io::Result<Self> {
        let mut pos = 0;
        let header = PidLogHeader::from_buf(bytes)?;
        pos += PidLogHeader::packed_footprint();
        let vec_of_entries = parse_to_vec::<PidLogEntry>(&mut &bytes[pos..]);

        Ok(Self {
            header,
            entries: vec_of_entries,
        })
    }

    pub fn entries(&self) -> &[PidLogEntry] {
        &self.entries
    }
}

impl std::fmt::Display for PidLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Header: {}", self.header)?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLogHeader {
    #[serde(with = "BigArray")]
    unique_description: [u8; 128],
    version: u16,
}

impl PidLogHeader {
    pub const UNIQUE_DESCRIPTION: &'static str = "MBED-MOTOR-CONTROL-PID-LOG";

    fn unique_description(&self) -> String {
        super::parse_unique_description(self.unique_description)
    }

    pub fn is_buf_header(bytes: &mut &[u8]) -> io::Result<bool> {
        let deserialized = Self::from_buf(bytes)?;
        let is_header = deserialized.unique_description() == Self::UNIQUE_DESCRIPTION;
        Ok(is_header)
    }
}

impl PidLogHeader {
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

impl std::fmt::Display for PidLogHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-v{}", self.unique_description(), self.version)
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLogEntry {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub rpm: f32,
    pub pid_err: f32,
    pub servo_duty_cycle: f32,
}

impl LogEntry for PidLogEntry {
    fn from_buf(bytes: &mut &[u8]) -> io::Result<Self> {
        let timestamp_ms = read_u32(bytes)?;
        let timestamp_ms_str = parse_timestamp(timestamp_ms);
        let rpm = read_f32(bytes)?;
        let pid_err = read_f32(bytes)?;
        let servo_duty_cycle = read_f32(bytes)?;

        Ok(Self {
            timestamp_ms_str,
            timestamp_ms,
            rpm,
            pid_err,
            servo_duty_cycle,
        })
    }

    fn packed_footprint() -> usize {
        mem::size_of::<u32>() // timestamp
            + mem::size_of::<f32>() // rpm 
            + mem::size_of::<f32>() // pid err
            + mem::size_of::<f32>() // servo duty cycle
    }

    fn timestamp_ms(&self) -> u32 {
        self.timestamp_ms
    }
}

impl std::fmt::Display for PidLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} {} {}",
            self.timestamp_ms, self.rpm, self.pid_err, self.servo_duty_cycle
        )
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_deserialize() {
        let data = fs::read("test_data/pid_20240906_081235_00.bin").unwrap();
        let pidlog = PidLog::from_buf(&mut data.as_slice()).unwrap();
        eprintln!("{}", pidlog.header);
        assert_eq!(
            pidlog.header.unique_description(),
            PidLogHeader::UNIQUE_DESCRIPTION
        );
        assert_eq!(pidlog.header.version, 0);
        let first_entry = pidlog.entries.first().unwrap();
        assert_eq!(first_entry.rpm, 0.0);
        assert_eq!(first_entry.pid_err, 1.0);
        assert_eq!(first_entry.servo_duty_cycle, 2.0);
        let second_entry = pidlog.entries.get(1).unwrap();
        assert_eq!(second_entry.rpm, 123.0);
        assert_eq!(second_entry.pid_err, 456.0);
        assert_eq!(second_entry.servo_duty_cycle, 789.0);
        //eprintln!("{pidlog}");
    }
}
