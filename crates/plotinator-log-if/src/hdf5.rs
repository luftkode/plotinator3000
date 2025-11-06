use std::path::Path;

use crate::prelude::Plotable;

/// A given HDF5 file should implement this
pub trait SkytemHdf5: Plotable + Sized {
    /// A descriptive name of what the implementer of [`SkytemHdf5`] is. e.g. "Bifrost Current" or "Njord Altimeter".
    const DESCRIPTIVE_NAME: &str;

    /// Take a path to an HDF5 file and parse it
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self>;
}
