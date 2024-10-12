use std::{fmt, io};

use crate::{util::parse_timestamp, LogEntry};
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PidLogEntry {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub rpm: f32,
    pub pid_output: f32,
    pub servo_duty_cycle: f32,
    pub rpm_error_count: u32,
    pub first_valid_rpm_count: u32,
}

impl LogEntry for PidLogEntry {
    fn from_reader(reader: &mut impl io::Read) -> io::Result<Self> {
        let timestamp_ms = reader.read_u32::<LittleEndian>()?;
        let timestamp_ms_str = parse_timestamp(timestamp_ms);
        let rpm = reader.read_f32::<LittleEndian>()?;
        let pid_output = reader.read_f32::<LittleEndian>()?;
        let servo_duty_cycle = reader.read_f32::<LittleEndian>()?;
        let rpm_error_count = reader.read_u32::<LittleEndian>()?;
        let first_valid_rpm_count = reader.read_u32::<LittleEndian>()?;

        Ok(Self {
            timestamp_ms_str,
            timestamp_ms,
            rpm,
            pid_output,
            servo_duty_cycle,
            rpm_error_count,
            first_valid_rpm_count,
        })
    }

    fn timestamp_ns(&self) -> f64 {
        (self.timestamp_ms as u64 * 1_000_000) as f64
    }
}

impl fmt::Display for PidLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {} {} {} {}",
            self.timestamp_ms,
            self.rpm,
            self.pid_output,
            self.servo_duty_cycle,
            self.rpm_error_count,
            self.first_valid_rpm_count
        )
    }
}
