use core::fmt;

use log_if::log::LogEntry;
use serde::{Deserialize, Serialize};
use v1::StatusLogEntryV1;
use v2::StatusLogEntryV2;

pub(crate) mod v1;
/// Only difference between v1 and v2 is changes to the motor state enum
pub(crate) mod v2;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum StatusLogEntry {
    V1(StatusLogEntryV1),
    V2(StatusLogEntryV2),
}

impl fmt::Display for StatusLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusLogEntry::V1(e) => write!(f, "{e}"),
            StatusLogEntry::V2(e) => write!(f, "{e}"),
        }
    }
}

impl StatusLogEntry {
    pub(crate) fn timestamp_ns(&self) -> f64 {
        match self {
            Self::V1(e) => e.timestamp_ns(),
            Self::V2(e) => e.timestamp_ns(),
        }
    }

    pub(crate) fn motor_state(&self) -> u8 {
        match self {
            Self::V1(e) => e.motor_state as u8,
            Self::V2(e) => e.motor_state as u8,
        }
    }

    pub(crate) fn motor_state_string(&self) -> String {
        match self {
            Self::V1(e) => e.motor_state.to_string(),
            Self::V2(e) => e.motor_state.to_string(),
        }
    }
}

pub(super) fn convert_v1_to_status_log_entry(v1: Vec<StatusLogEntryV1>) -> Vec<StatusLogEntry> {
    v1.into_iter().map(StatusLogEntry::V1).collect()
}

pub(super) fn convert_v2_to_status_log_entry(v2: Vec<StatusLogEntryV2>) -> Vec<StatusLogEntry> {
    v2.into_iter().map(StatusLogEntry::V2).collect()
}
