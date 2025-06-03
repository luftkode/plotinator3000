use chrono::{DateTime, Utc};
use plotinator_hdf5::{bifrost::BifrostLoopCurrent, wasp200::Wasp200};
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a supported HDF5 format, which can be any of the supported HDF5 format types.
///
/// This simply serves to encapsulate all the supported HDF5 formats in a single type
macro_rules! define_supported_hdf5_formats {
    ( $( $variant:ident => $ty:ty ),* $(,)? ) => {
        #[derive(Debug, Clone, Deserialize, Serialize)]
        pub enum SupportedHdfFormat {
            $( $variant($ty), )*
        }

        $(
            impl From<$ty> for SupportedHdfFormat {
                fn from(value: $ty) -> Self {
                    SupportedHdfFormat::$variant(value)
                }
            }
        )*

        impl Plotable for SupportedHdfFormat {
            fn raw_plots(&self) -> &[RawPlot] {
                match self {
                    $( SupportedHdfFormat::$variant(inner) => inner.raw_plots(), )*
                }
            }

            fn first_timestamp(&self) -> DateTime<Utc> {
                match self {
                    $( SupportedHdfFormat::$variant(inner) => inner.first_timestamp(), )*
                }
            }

            fn descriptive_name(&self) -> &str {
                match self {
                    $( SupportedHdfFormat::$variant(inner) => inner.descriptive_name(), )*
                }
            }

            fn labels(&self) -> Option<&[PlotLabels]> {
                match self {
                    $( SupportedHdfFormat::$variant(inner) => inner.labels(), )*
                }
            }

            fn metadata(&self) -> Option<Vec<(String, String)>> {
                match self {
                    $( SupportedHdfFormat::$variant(inner) => inner.metadata(), )*
                }
            }
        }
    };
}

define_supported_hdf5_formats! {
    BifrostLoopCurrent => BifrostLoopCurrent,
    Wasp200Height => Wasp200,
}
