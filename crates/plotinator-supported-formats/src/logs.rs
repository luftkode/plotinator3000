use std::io;

use crate::parse_info::{ParseInfo, ParsedBytes, TotalBytes};
use chrono::{DateTime, Utc};
use plotinator_log_if::prelude::*;
use plotinator_logs::{
    generator::GeneratorLog,
    inclinometer_sps::InclinometerSps,
    mbed_motor_control::{pid::pidlog::PidLog, status::statuslog::StatusLog},
    navsys::NavSysSps,
    navsys_kitchen_sink::NavSysSpsKitchenSink,
};
use plotinator_ui_file_io::{ParseUpdate, UpdateChannel};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a supported log format, which can be any of the supported log format types.
///
/// This simply serves to encapsulate all the supported log formats in a single type
macro_rules! define_supported_log_formats {
    ( $( $variant:ident => $ty:ty ),* $(,)? ) => {
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub enum SupportedLog {

            $( $variant($ty, ParseInfo), )*
        }

        impl std::fmt::Display for SupportedLog {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$variant(supported_log, _) => write!(f, "{}", supported_log.descriptive_name()), )*
                }
            }
        }

        impl SupportedLog {
            pub(crate) fn parse_info(&self) -> ParseInfo {
                match self {
                    $( Self::$variant(_, parse_info) => *parse_info, )*
                }
            }

            pub(crate) fn parse_from_buf(content: &[u8], path: PathBuf, tx: UpdateChannel) -> io::Result<Self> {
                let total_bytes = content.len();
                log::debug!("Parsing content of length: {total_bytes}");

                $(
                    log::debug!("Attempting to parse as {}", stringify!($ty));
                    tx.send(ParseUpdate::Attempting {
                        path: path.to_owned(),
                        format_name: <$ty>::DESCRIPTIVE_NAME.to_owned(),
                    });
                    match <$ty>::try_from_buf(content) {
                        Ok((log_data, read_bytes)) => {
                            log::debug!("Successfully parsed as {}", stringify!($ty));
                            log::debug!("Read: {read_bytes} bytes");
                            let parse_info = ParseInfo::new(
                                ParsedBytes(read_bytes),
                                TotalBytes(total_bytes)
                            );
                            let log = Self::$variant(log_data, parse_info);
                            log::debug!("Got: {}", log.descriptive_name());
                            tx.send(ParseUpdate::Confirmed {
                                path: path.to_owned(),
                                format_name: <$ty>::DESCRIPTIVE_NAME.to_owned(),
                            });
                            return Ok(log);
                        }
                        Err(e) => {
                            log::debug!("Failed to parse as {}: {e}", stringify!($ty));
                        }
                    }
                )*

                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Unrecognized format",
                ))
            }
        }

        $(
            impl From<($ty, ParseInfo)> for SupportedLog {
                fn from(value: ($ty, ParseInfo)) -> Self {
                    Self::$variant(value.0, value.1)
                }
            }
        )*

        impl Plotable for SupportedLog {
            fn raw_plots(&self) -> &[RawPlot] {
                match self {
                    $( Self::$variant(l, _) => l.raw_plots(), )*
                }
            }

            fn first_timestamp(&self) -> DateTime<Utc> {
                match self {
                    $( Self::$variant(l, _) => l.first_timestamp(), )*
                }
            }

            fn descriptive_name(&self) -> &str {
                match self {
                    $( Self::$variant(l, _) => l.descriptive_name(), )*
                }
            }

            fn labels(&self) -> Option<&[PlotLabels]> {
                match self {
                    $( Self::$variant(l, _) => l.labels(), )*
                }
            }

            fn metadata(&self) -> Option<Vec<(String, String)>> {
                match self {
                    $( Self::$variant(l, _) => l.metadata(), )*
                }
            }
        }
    };
}

define_supported_log_formats! {
    MbedPid => PidLog,
    MbedStatus => StatusLog,
    Generator => GeneratorLog,
    NavSysSps => NavSysSps,
    NavSysSpsKitchenSink => NavSysSpsKitchenSink,
    InclinometerSps => InclinometerSps,
}
