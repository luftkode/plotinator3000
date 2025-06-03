use std::io;

use chrono::{DateTime, Utc};
use parse_info::ParseInfo;
use plotinator_log_if::prelude::*;
use plotinator_logs::{
    generator::GeneratorLog,
    mbed_motor_control::{pid::pidlog::PidLog, status::statuslog::StatusLog},
    navsys::NavSysSps,
    wasp200::Wasp200Sps,
};
use serde::{Deserialize, Serialize};

use crate::app::supported_format::logs::parse_info::{ParsedBytes, TotalBytes};

pub(crate) mod parse_info;

/// Represents a supported log format, which can be any of the supported log format types.
///
/// This simply serves to encapsulate all the supported log format in a single type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SupportedLog {
    MbedPid(PidLog, ParseInfo),
    MbedStatus(StatusLog, ParseInfo),
    Generator(GeneratorLog, ParseInfo),
    NavSysSps(NavSysSps, ParseInfo),
    Wasp200Sps(Wasp200Sps, ParseInfo),
}

impl SupportedLog {
    pub(crate) fn parse_info(&self) -> ParseInfo {
        match self {
            Self::MbedPid(_, parse_info)
            | Self::MbedStatus(_, parse_info)
            | Self::NavSysSps(_, parse_info)
            | Self::Generator(_, parse_info)
            | Self::Wasp200Sps(_, parse_info) => *parse_info,
        }
    }

    pub(crate) fn parse_from_buf(content: &[u8]) -> io::Result<Self> {
        let total_bytes = content.len();
        log::debug!("Parsing content of length: {total_bytes}");
        let log: Self = if let Ok((pidlog, read_bytes)) = PidLog::try_from_buf(content) {
            log::debug!("Read: {read_bytes} bytes");
            (
                pidlog,
                ParseInfo::new(ParsedBytes(read_bytes), TotalBytes(total_bytes)),
            )
                .into()
        } else if let Ok((statuslog, read_bytes)) = StatusLog::try_from_buf(content) {
            (
                statuslog,
                ParseInfo::new(ParsedBytes(read_bytes), TotalBytes(read_bytes)),
            )
                .into()
        } else if let Ok((genlog, read_bytes)) = GeneratorLog::try_from_buf(content) {
            (
                genlog,
                ParseInfo::new(ParsedBytes(read_bytes), TotalBytes(total_bytes)),
            )
                .into()
        } else if let Ok((navsyssps_log, read_bytes)) = NavSysSps::try_from_buf(content) {
            (
                navsyssps_log,
                ParseInfo::new(ParsedBytes(read_bytes), TotalBytes(total_bytes)),
            )
                .into()
        } else if let Ok((wasp200sps, read_bytes)) = Wasp200Sps::try_from_buf(content) {
            (
                wasp200sps,
                ParseInfo::new(ParsedBytes(read_bytes), TotalBytes(total_bytes)),
            )
                .into()
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unrecognized format",
            ));
        };
        log::debug!("Got: {}", log.descriptive_name());
        Ok(log)
    }
}

impl From<(PidLog, ParseInfo)> for SupportedLog {
    fn from(value: (PidLog, ParseInfo)) -> Self {
        Self::MbedPid(value.0, value.1)
    }
}

impl From<(StatusLog, ParseInfo)> for SupportedLog {
    fn from(value: (StatusLog, ParseInfo)) -> Self {
        Self::MbedStatus(value.0, value.1)
    }
}

impl From<(GeneratorLog, ParseInfo)> for SupportedLog {
    fn from(value: (GeneratorLog, ParseInfo)) -> Self {
        Self::Generator(value.0, value.1)
    }
}

impl From<(NavSysSps, ParseInfo)> for SupportedLog {
    fn from(value: (NavSysSps, ParseInfo)) -> Self {
        Self::NavSysSps(value.0, value.1)
    }
}

impl From<(Wasp200Sps, ParseInfo)> for SupportedLog {
    fn from(value: (Wasp200Sps, ParseInfo)) -> Self {
        Self::Wasp200Sps(value.0, value.1)
    }
}

impl Plotable for SupportedLog {
    fn raw_plots(&self) -> &[RawPlot] {
        match self {
            Self::MbedPid(l, _) => l.raw_plots(),
            Self::MbedStatus(l, _) => l.raw_plots(),
            Self::Generator(l, _) => l.raw_plots(),
            Self::NavSysSps(l, _) => l.raw_plots(),
            Self::Wasp200Sps(l, _) => l.raw_plots(),
        }
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::MbedPid(l, _) => l.first_timestamp(),
            Self::MbedStatus(l, _) => l.first_timestamp(),
            Self::Generator(l, _) => l.first_timestamp(),
            Self::NavSysSps(l, _) => l.first_timestamp(),
            Self::Wasp200Sps(l, _) => l.first_timestamp(),
        }
    }

    fn descriptive_name(&self) -> &str {
        match self {
            Self::MbedPid(l, _) => l.descriptive_name(),
            Self::MbedStatus(l, _) => l.descriptive_name(),
            Self::Generator(l, _) => l.descriptive_name(),
            Self::NavSysSps(l, _) => l.descriptive_name(),
            Self::Wasp200Sps(l, _) => l.descriptive_name(),
        }
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        match self {
            Self::MbedPid(l, _) => l.labels(),
            Self::MbedStatus(l, _) => l.labels(),
            Self::Generator(l, _) => l.labels(),
            Self::NavSysSps(l, _) => l.labels(),
            Self::Wasp200Sps(l, _) => l.labels(),
        }
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        match self {
            Self::MbedPid(l, _) => l.metadata(),
            Self::MbedStatus(l, _) => l.metadata(),
            Self::Generator(l, _) => l.metadata(),
            Self::NavSysSps(l, _) => l.metadata(),
            Self::Wasp200Sps(l, _) => l.metadata(),
        }
    }
}
