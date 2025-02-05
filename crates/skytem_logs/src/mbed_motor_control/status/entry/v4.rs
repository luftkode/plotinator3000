use std::{fmt, io};

use crate::util::parse_timestamp;
use byteorder::{LittleEndian, ReadBytesExt};
use derive_more::derive::Display;
use log_if::log::LogEntry;
use serde::{Deserialize, Serialize};
use strum_macros::FromRepr;

#[allow(
    non_camel_case_types,
    clippy::upper_case_acronyms,
    reason = "This is how it is represented in the motor control code"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, FromRepr, Display)]
pub enum MotorState {
    POWER_HOLD = 0,
    ECU_ON_WAIT_PUMP,
    ECU_ON_WAIT_PRESS_START,
    DO_IGNITION,
    IGNITION_END,
    IDLE,
    STANDBY_WAIT_FOR_CAP,
    STANDBY_WAIT_FOR_T_RUN,
    GOTO_RUN_WHEN_T_RUN,
    STANDBY_READY,
    RUNNING,
    WAIT_TIME_SHUTDOWN,
    INVALID_STATE,
    CONFIGURING,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StatusLogEntryV4 {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub engine_temp: f32,
    pub setpoint: f32,
    pub motor_state: MotorState,
    pub runtime_s: u32,
}

impl fmt::Display for StatusLogEntryV4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {} {} {}",
            self.timestamp_ms, self.engine_temp, self.setpoint, self.motor_state, self.runtime_s
        )
    }
}

impl LogEntry for StatusLogEntryV4 {
    fn from_reader(reader: &mut impl io::BufRead) -> io::Result<(Self, usize)> {
        // Start with 0 bytes read
        let mut total_bytes_read = 0;

        // Read and track the number of bytes read after each operation
        let timestamp_ms = reader.read_u32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&timestamp_ms);

        let timestamp_ms_str = parse_timestamp(timestamp_ms);

        let engine_temp = reader.read_f32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&engine_temp);

        let setpoint = reader.read_f32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&setpoint);

        // Handle MotorState with size tracking for the u8 used
        let motor_state_byte = reader.read_u8()?;
        total_bytes_read += size_of_val(&motor_state_byte);
        let Some(motor_state) = MotorState::from_repr(motor_state_byte.into()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid motor state: {motor_state_byte}"),
            ));
        };

        let runtime_s = reader.read_u32::<LittleEndian>()?;
        total_bytes_read += size_of_val(&runtime_s);

        // Return the instance and total bytes read
        Ok((
            Self {
                timestamp_ms_str,
                timestamp_ms,
                engine_temp,
                setpoint,
                motor_state,
                runtime_s,
            },
            total_bytes_read,
        ))
    }

    fn timestamp_ns(&self) -> f64 {
        (self.timestamp_ms as u64 * 1_000_000) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motor_state_deserialize() {
        assert_eq!(
            MotorState::DO_IGNITION,
            MotorState::from_repr(3).expect("Value doesn't map to variant")
        );
        assert_eq!(
            MotorState::RUNNING,
            MotorState::from_repr(10).expect("Value doesn't map to variant")
        );
    }
}
