use crate::logs::{parse_to_vec, Log, LogEntry};
use crate::util::parse_timestamp;
use byteorder::{LittleEndian, ReadBytesExt};
use serde_big_array::BigArray;
use std::io;
use strum_macros::{Display, FromRepr};

use super::MbedMotorControlLogHeader;

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
pub struct StatusLog {
    header: StatusLogHeader,
    entries: Vec<StatusLogEntry>,
    timestamps_with_state_changes: Vec<(u32, MotorState)>, // for memoization
}

impl Log for StatusLog {
    type Entry = StatusLogEntry;

    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = StatusLogHeader::from_reader(reader)?;
        let vec_of_entries: Vec<StatusLogEntry> = parse_to_vec(reader);
        let timestamps_with_state_changes = parse_timestamps_with_state_changes(&vec_of_entries);
        Ok(Self {
            header,
            entries: vec_of_entries,
            timestamps_with_state_changes,
        })
    }

    fn entries(&self) -> &[Self::Entry] {
        &self.entries
    }
}

impl StatusLog {
    pub fn timestamps_with_state_changes(&self) -> &[(u32, MotorState)] {
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

fn parse_timestamps_with_state_changes(entries: &[StatusLogEntry]) -> Vec<(u32, MotorState)> {
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

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StatusLogHeader {
    #[serde(with = "BigArray")]
    unique_description: [u8; 128],
    version: u16,
}

impl MbedMotorControlLogHeader for StatusLogHeader {
    const UNIQUE_DESCRIPTION: &'static str = "MBED-MOTOR-CONTROL-STATUS-LOG";

    fn unique_description_bytes(&self) -> &[u8; 128] {
        &self.unique_description
    }

    fn version(&self) -> u16 {
        self.version
    }

    fn new(unique_description: [u8; 128], version: u16) -> Self {
        StatusLogHeader {
            unique_description,
            version,
        }
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
    pub motor_state: MotorState,
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
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let timestamp_ms = reader.read_u32::<LittleEndian>()?;
        let timestamp_ms_str = parse_timestamp(timestamp_ms);
        let engine_temp = reader.read_f32::<LittleEndian>()?;
        let fan_on = reader.read_u8()? == 1;
        let vbat = reader.read_f32::<LittleEndian>()?;
        let setpoint = reader.read_f32::<LittleEndian>()?;
        let motor_state = match MotorState::from_repr(reader.read_u8()?.into()) {
            Some(st) => st,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid motor state",
                ))
            }
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

    fn timestamp_ms(&self) -> u32 {
        self.timestamp_ms
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use testresult::TestResult;

    use super::*;

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read("test_data/fake_controlled_data/status_20240906_081236_00.bin")?;
        let status_log = StatusLog::from_reader(&mut data.as_slice())?;
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
        assert_eq!(first_entry.motor_state, MotorState::IGNITION_END);
        let second_entry = status_log.entries.get(1).unwrap();
        assert_eq!(second_entry.engine_temp, 123.4);
        assert_eq!(second_entry.fan_on, false);
        assert_eq!(second_entry.vbat, 2.34);
        assert_eq!(second_entry.setpoint, 3.45);
        assert_eq!(second_entry.motor_state, MotorState::WAIT_FOR_T_STANDBY);
        //eprintln!("{status_log}");
        Ok(())
    }

    #[test]
    fn test_motor_state_deserialize() -> TestResult {
        assert_eq!(MotorState::DO_IGNITION, MotorState::from_repr(3).unwrap());
        assert_eq!(
            MotorState::WAIT_TIME_SHUTDOWN,
            MotorState::from_repr(10).unwrap()
        );
        Ok(())
    }
}
