use chrono::{DateTime, Utc};
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};
use skytem_hdf5::{bifrost::BifrostLoopCurrent, wasp200::Wasp200};

/// Represents a supported HDF5 format, which can be any of the supported HDF5 format types.
///
/// This simply serves to encapsulate all the supported HDF5 formats in a single type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SupportedHdfFormat {
    BifrostLoopCurrent(BifrostLoopCurrent),
    Wasp200Height(Wasp200),
}

impl From<BifrostLoopCurrent> for SupportedHdfFormat {
    fn from(value: BifrostLoopCurrent) -> Self {
        Self::BifrostLoopCurrent(value)
    }
}

impl From<Wasp200> for SupportedHdfFormat {
    fn from(value: Wasp200) -> Self {
        Self::Wasp200Height(value)
    }
}

impl Plotable for SupportedHdfFormat {
    fn raw_plots(&self) -> &[RawPlot] {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.raw_plots(),
            Self::Wasp200Height(hdf) => hdf.raw_plots(),
        }
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.first_timestamp(),
            Self::Wasp200Height(hdf) => hdf.first_timestamp(),
        }
    }

    fn descriptive_name(&self) -> &str {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.descriptive_name(),
            Self::Wasp200Height(hdf) => hdf.descriptive_name(),
        }
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.labels(),
            Self::Wasp200Height(hdf) => hdf.labels(),
        }
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.metadata(),
            Self::Wasp200Height(hdf) => hdf.metadata(),
        }
    }
}
