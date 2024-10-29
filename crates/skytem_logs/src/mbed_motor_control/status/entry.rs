use log_if::log::LogEntry;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use v1::StatusLogEntryV1;
use v2::StatusLogEntryV2;

pub(crate) mod v1;
/// Only difference between v1 and v2 is changes to the motor state enum
pub(crate) mod v2;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Display)]
pub(crate) enum StatusLogEntry {
    V1(StatusLogEntryV1),
    V2(StatusLogEntryV2),
}

impl StatusLogEntry {
    pub(crate) fn timestamp_ns(&self) -> f64 {
        match self {
            StatusLogEntry::V1(e) => e.timestamp_ns(),
            StatusLogEntry::V2(e) => e.timestamp_ns(),
        }
    }

    pub(crate) fn motor_state(&self) -> u8 {
        match self {
            StatusLogEntry::V1(e) => e.motor_state as u8,
            StatusLogEntry::V2(e) => e.motor_state as u8,
        }
    }

    pub(crate) fn motor_state_string(&self) -> String {
        match self {
            StatusLogEntry::V1(e) => e.motor_state.to_string(),
            StatusLogEntry::V2(e) => e.motor_state.to_string(),
        }
    }
}

pub(super) fn convert_v1_to_status_log_entry(v1: Vec<StatusLogEntryV1>) -> Vec<StatusLogEntry> {
    v1.into_iter().map(StatusLogEntry::V1).collect()
}

pub(super) fn convert_v2_to_status_log_entry(v2: Vec<StatusLogEntryV2>) -> Vec<StatusLogEntry> {
    v2.into_iter().map(StatusLogEntry::V2).collect()
}
