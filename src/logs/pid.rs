use std::{io, mem};

use crate::util::{parse_timestamp, read_f32, read_u32};

use super::LogEntry;

#[derive(Debug)]
pub struct PidLogEntry {
    timestamp_ms: String,
    rpm: f32,
    pid_err: f32,
    servo_duty_cycle: f32,
}

impl LogEntry for PidLogEntry {
    fn from_buf(bytes: &mut &[u8]) -> io::Result<Self> {
        let timestamp_ms = parse_timestamp(read_u32(bytes)?);
        let rpm = read_f32(bytes)?;
        let pid_err = read_f32(bytes)?;
        let servo_duty_cycle = read_f32(bytes)?;

        Ok(Self {
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
