use chrono::{DateTime, Utc};
use log_if::prelude::*;
use serde::{Deserialize, Serialize};
use skytem_hdf5::bifrost::BifrostLoopCurrent;

/// Represents a supported HDF5 format, which can be any of the supported HDF5 format types.
///
/// This simply serves to encapsulate all the supported HDF5 formats in a single type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SupportedHdfFormat {
    BifrostLoopCurrent(BifrostLoopCurrent),
}

impl From<BifrostLoopCurrent> for SupportedHdfFormat {
    fn from(value: BifrostLoopCurrent) -> Self {
        Self::BifrostLoopCurrent(value)
    }
}

impl Plotable for SupportedHdfFormat {
    fn raw_plots(&self) -> &[RawPlot] {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.raw_plots(),
        }
    }

    fn first_timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.first_timestamp(),
        }
    }

    fn descriptive_name(&self) -> &str {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.descriptive_name(),
        }
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.labels(),
        }
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        match self {
            Self::BifrostLoopCurrent(hdf) => hdf.metadata(),
        }
    }
}
