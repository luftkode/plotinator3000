use core::fmt;

use log_if::log::LogEntry;
use serde::{Deserialize, Serialize};
use v1::StatusLogEntryV1;
use v2::StatusLogEntryV2;
use v3::StatusLogEntryV3;
use v4::StatusLogEntryV4;

pub(crate) mod v1;
/// Only difference between v1 and v2 is changes to the motor state enum
pub(crate) mod v2;
/// Now entries include the runtime counter
pub(crate) mod v3;
/// Now the `vbat` and `fan_on` entry is moved to the high res (pid) log
pub(crate) mod v4;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum StatusLogEntry {
    V1(StatusLogEntryV1),
    V2(StatusLogEntryV2),
    V3(StatusLogEntryV3),
    V4(StatusLogEntryV4),
}

impl fmt::Display for StatusLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1(e) => write!(f, "{e}"),
            Self::V2(e) => write!(f, "{e}"),
            Self::V3(e) => write!(f, "{e}"),
            Self::V4(e) => write!(f, "{e}"),
        }
    }
}

impl StatusLogEntry {
    pub(crate) fn timestamp_ns(&self) -> f64 {
        match self {
            Self::V1(e) => e.timestamp_ns(),
            Self::V2(e) => e.timestamp_ns(),
            Self::V3(e) => e.timestamp_ns(),
            Self::V4(e) => e.timestamp_ns(),
        }
    }

    pub(crate) fn motor_state(&self) -> u8 {
        match self {
            Self::V1(e) => e.motor_state as u8,
            Self::V2(e) => e.motor_state as u8,
            Self::V3(e) => e.motor_state as u8,
            Self::V4(e) => e.motor_state as u8,
        }
    }

    pub(crate) fn motor_state_string(&self) -> String {
        match self {
            Self::V1(e) => e.motor_state.to_string(),
            Self::V2(e) => e.motor_state.to_string(),
            Self::V3(e) => e.motor_state.to_string(),
            Self::V4(e) => e.motor_state.to_string(),
        }
    }
}

pub(super) fn convert_v1_to_status_log_entry(v1: Vec<StatusLogEntryV1>) -> Vec<StatusLogEntry> {
    v1.into_iter().map(StatusLogEntry::V1).collect()
}

pub(super) fn convert_v2_to_status_log_entry(v2: Vec<StatusLogEntryV2>) -> Vec<StatusLogEntry> {
    v2.into_iter().map(StatusLogEntry::V2).collect()
}

pub(super) fn convert_v3_to_status_log_entry(v2: Vec<StatusLogEntryV3>) -> Vec<StatusLogEntry> {
    v2.into_iter().map(StatusLogEntry::V3).collect()
}

pub(super) fn convert_v4_to_status_log_entry(v2: Vec<StatusLogEntryV4>) -> Vec<StatusLogEntry> {
    v2.into_iter().map(StatusLogEntry::V4).collect()
}
