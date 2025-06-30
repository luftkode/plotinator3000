use chrono::{DateTime, Utc};
use plotinator_hdf5::frame_altimeters::FrameAltimeters;
use plotinator_hdf5::njord_ins::NjordIns;
use plotinator_hdf5::{bifrost::BifrostLoopCurrent, wasp200::Wasp200};
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// Represents a supported HDF5 format, which can be any of the supported HDF5 format types.
///
/// This simply serves to encapsulate all the supported HDF5 formats in a single type
macro_rules! define_supported_hdf5_formats {
    ( $( $variant:ident => $ty:ty ),* $(,)? ) => {
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub enum SupportedHdf5Format {
            $( $variant($ty), )*
        }

        $(
            impl From<$ty> for SupportedHdf5Format {
                fn from(value: $ty) -> Self {
                    SupportedHdf5Format::$variant(value)
                }
            }
        )*

        impl SkytemHdf5 for SupportedHdf5Format {
            fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
                let path = path.as_ref();

                // Try each supported format in order
                $(
                    if let Ok(format_data) = <$ty>::from_path(path) {
                        return Ok(SupportedHdf5Format::$variant(format_data));
                    }
                )*

                // If none of the formats worked, return an error
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Unrecognized HDF5 file format",
                ))
            }
        }

        impl Plotable for SupportedHdf5Format {
            fn raw_plots(&self) -> &[RawPlot] {
                match self {
                    $( SupportedHdf5Format::$variant(inner) => inner.raw_plots(), )*
                }
            }

            fn first_timestamp(&self) -> DateTime<Utc> {
                match self {
                    $( SupportedHdf5Format::$variant(inner) => inner.first_timestamp(), )*
                }
            }

            fn descriptive_name(&self) -> &str {
                match self {
                    $( SupportedHdf5Format::$variant(inner) => inner.descriptive_name(), )*
                }
            }

            fn labels(&self) -> Option<&[PlotLabels]> {
                match self {
                    $( SupportedHdf5Format::$variant(inner) => inner.labels(), )*
                }
            }

            fn metadata(&self) -> Option<Vec<(String, String)>> {
                match self {
                    $( SupportedHdf5Format::$variant(inner) => inner.metadata(), )*
                }
            }
        }
    };
}

define_supported_hdf5_formats! {
    BifrostLoopCurrent => BifrostLoopCurrent,
    Wasp200Height => Wasp200,
    FrameAltimeters => FrameAltimeters,
    NjordIns => NjordIns
}
