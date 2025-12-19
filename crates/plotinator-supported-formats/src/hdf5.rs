use chrono::{DateTime, Utc};
use plotinator_log_if::prelude::*;
use plotinator_ui_file_io::{ParseUpdate, UpdateChannel};
use serde::{Deserialize, Serialize};
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

        impl std::fmt::Display for SupportedHdf5Format {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$variant(supported_hdf5_format) => write!(f, "{}", supported_hdf5_format.descriptive_name()), )*
                }
            }
        }

        $(
            impl From<$ty> for SupportedHdf5Format {
                fn from(value: $ty) -> Self {
                    SupportedHdf5Format::$variant(value)
                }
            }
        )*

        impl SupportedHdf5Format {
            pub fn from_path(path: impl AsRef<Path>, tx: UpdateChannel) -> anyhow::Result<Self> {
                let path = path.as_ref();

                // Try each supported format in order
                $(
                    tx.send(ParseUpdate::Attempting {
                        path: path.to_path_buf(),
                        format_name: <$ty>::DESCRIPTIVE_NAME.to_owned(),
                    });
                    match <$ty>::from_path(path) {
                        Ok(format_data) => {
                            tx.send(ParseUpdate::Confirmed {
                                path: path.to_path_buf(),
                                format_name: <$ty>::DESCRIPTIVE_NAME.to_owned(),
                            });
                            return Ok(SupportedHdf5Format::$variant(format_data));
                        }
                        Err(e) => log::debug!("Not '{}' compatible: {e}", <$ty>::DESCRIPTIVE_NAME),
                    }
                )*

                // If none of the formats worked, return an error
                anyhow::bail!("Unrecognized HDF5 file format");
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
    BifrostLoopCurrent => plotinator_hdf5::bifrost::BifrostLoopCurrent,
    NjordAltimeter => plotinator_hdf5::NjordAltimeter,
    FrameAltimeters => plotinator_hdf5::frame_altimeters::FrameAltimeters,
    FrameInclinometers => plotinator_hdf5::frame_inclinometers::FrameInclinometers,
    FrameMagnetometer => plotinator_hdf5::frame_magnetometer::FrameMagnetometer,
    FrameGps => plotinator_hdf5::frame_gps::FrameGps,
    NjordIns => plotinator_hdf5::njord_ins::NjordIns,
    Tsc => plotinator_hdf5::tsc::Tsc,
    Altimeter => plotinator_hdf5::altimeter::Altimeter,
    AltimeterMinMax => plotinator_hdf5::altimeter_minmax::AltimeterMinMax,
    Inclinometer => plotinator_hdf5::inclinometer::Inclinometer,
}
