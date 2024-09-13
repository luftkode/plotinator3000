use std::io;

use crate::{logs::LogEntry, util::parse_timestamp};
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLogEntry {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub rpm: f32,
    pub pid_err: f32,
    pub servo_duty_cycle: f32,
}

impl PidLogEntry {
    pub fn timestamp_ms(&self) -> u32 {
        self.timestamp_ms
    }
}

impl LogEntry for PidLogEntry {
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let timestamp_ms = reader.read_u32::<LittleEndian>()?;
        let timestamp_ms_str = parse_timestamp(timestamp_ms);
        let rpm = reader.read_f32::<LittleEndian>()?;
        let pid_err = reader.read_f32::<LittleEndian>()?;
        let servo_duty_cycle = reader.read_f32::<LittleEndian>()?;

        Ok(Self {
            timestamp_ms_str,
            timestamp_ms,
            rpm,
            pid_err,
            servo_duty_cycle,
        })
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
