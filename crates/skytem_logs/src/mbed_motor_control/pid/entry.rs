pub(crate) mod v1;
pub(crate) mod v2;
pub(crate) mod v3;

use std::fmt;

use log_if::log::LogEntry as _;
use serde::{Deserialize, Serialize};
use v1::PidLogEntryV1;
use v2::PidLogEntryV2;
use v3::PidLogEntryV3;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PidLogEntry {
    V1(PidLogEntryV1),
    V2(PidLogEntryV2),
    V3(PidLogEntryV3),
}

impl fmt::Display for PidLogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V1(e) => write!(f, "{e}"),
            Self::V2(e) => write!(f, "{e}"),
            Self::V3(e) => write!(f, "{e}"),
        }
    }
}

impl PidLogEntry {
    pub(crate) fn timestamp_ns(&self) -> f64 {
        match self {
            Self::V1(e) => e.timestamp_ns(),
            Self::V2(e) => e.timestamp_ns(),
            Self::V3(e) => e.timestamp_ns(),
        }
    }
}

pub(super) fn convert_v1_to_pid_log_entry(v1: Vec<PidLogEntryV1>) -> Vec<PidLogEntry> {
    v1.into_iter().map(PidLogEntry::V1).collect()
}
pub(super) fn convert_v2_to_pid_log_entry(v2: Vec<PidLogEntryV2>) -> Vec<PidLogEntry> {
    v2.into_iter().map(PidLogEntry::V2).collect()
}
pub(super) fn convert_v3_to_pid_log_entry(v3: Vec<PidLogEntryV3>) -> Vec<PidLogEntry> {
    v3.into_iter().map(PidLogEntry::V3).collect()
}
