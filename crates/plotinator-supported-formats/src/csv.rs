use crate::parse_info::{ParseInfo, ParsedBytes, TotalBytes};
use chrono::{DateTime, Utc};
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};
use std::io;

/// Represents a supported csv format.
///
/// This simply serves to encapsulate all the supported csv formats in a single type
macro_rules! define_supported_csv_formats {
    ( $( $variant:ident => $ty:ty ),* $(,)? ) => {
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub enum SupportedCsv {

            $( $variant($ty, ParseInfo), )*
        }

        impl SupportedCsv {
            pub(crate) fn parse_info(&self) -> ParseInfo {
                match self {
                    $( Self::$variant(_, parse_info) => *parse_info, )*
                }
            }

            pub(crate) fn parse_from_buf(content: &[u8]) -> io::Result<Self> {
                let total_bytes = content.len();
                log::debug!("Parsing content of length: {total_bytes}");

                $(
                    log::debug!("Attempting to parse as {}", stringify!($ty));
                    match <$ty>::try_from_buf(content) {
                        Ok((log_data, read_bytes)) => {
                            log::debug!("Successfully parsed as {}", stringify!($ty));
                            log::debug!("Read: {read_bytes} bytes");
                            let parse_info = ParseInfo::new(
                                ParsedBytes(read_bytes),
                                TotalBytes(total_bytes)
                            );
                            let csv = Self::$variant(log_data, parse_info);
                            log::debug!("Got: {}", csv.descriptive_name());
                            return Ok(csv);
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
            impl From<($ty, ParseInfo)> for SupportedCsv {
                fn from(value: ($ty, ParseInfo)) -> Self {
                    Self::$variant(value.0, value.1)
                }
            }
        )*

        impl Plotable for SupportedCsv {
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

define_supported_csv_formats! {
    NjordInsPPP => plotinator_csv::njord_ins::NjordInsPPP,
    GrafNavPPP => plotinator_csv::grafnav::GrafNavPPP,
}
