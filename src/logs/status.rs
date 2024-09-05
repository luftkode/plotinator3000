use super::LogEntry;
use crate::util::{parse_timestamp, read_f32, read_u32};
use std::{io, mem};

#[derive(Debug)]
pub struct StatusLogEntry {
    timestamp_ms: String,
    engine_temp: f32,
    fan_on: bool,
    vbat: f32,
    setpoint: f32,
    motor_state: u8,
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
        Ok(Self {
            timestamp_ms: parse_timestamp(read_u32(bytes)?),
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
}
