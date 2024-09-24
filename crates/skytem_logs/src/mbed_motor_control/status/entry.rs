use std::{fmt, io};

use crate::{util::parse_timestamp, LogEntry};
use byteorder::{LittleEndian, ReadBytesExt};
use strum_macros::{Display, FromRepr};

#[allow(non_camel_case_types)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, FromRepr, Display,
)]
pub enum MotorState {
    POWER_HOLD = 0,
    ECU_ON_WAIT_PUMP,
    ECU_ON_WAIT_PRESS_START,
    DO_IGNITION,
    IGNITION_END,
    WAIT_FOR_T_STANDBY,
    STANDBY_WAIT_FOR_CAP,
    STANDBY_WAIT_FOR_T_RUN,
    STANDBY_READY,
    RUNNING,
    WAIT_TIME_SHUTDOWN,
    INVALID_STATE,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLogEntry {
    timestamp_ms_str: String,
    pub timestamp_ms: u32,
    pub engine_temp: f32,
    pub fan_on: bool,
    pub vbat: f32,
    pub setpoint: f32,
    pub motor_state: MotorState,
}

impl fmt::Display for StatusLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let timestamp_ms = reader.read_u32::<LittleEndian>()?;
        let timestamp_ms_str = parse_timestamp(timestamp_ms);
        let engine_temp = reader.read_f32::<LittleEndian>()?;
        let fan_on = reader.read_u8()? == 1;
        let vbat = reader.read_f32::<LittleEndian>()?;
        let setpoint = reader.read_f32::<LittleEndian>()?;
        let Some(motor_state) = MotorState::from_repr(reader.read_u8()?.into()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid motor state",
            ));
        };
        Ok(Self {
            timestamp_ms_str,
            timestamp_ms,
            engine_temp,
            fan_on,
            vbat,
            setpoint,
            motor_state,
        })
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
            MotorState::WAIT_TIME_SHUTDOWN,
            MotorState::from_repr(10).expect("Value doesn't map to variant")
        );
    }
}
