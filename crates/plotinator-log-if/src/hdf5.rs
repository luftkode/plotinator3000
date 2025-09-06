use std::path::Path;

use crate::prelude::Plotable;

/// A given HDF5 file should implement this
pub trait SkytemHdf5: Plotable + Sized {
    /// Take a path to an HDF5 file and parse it
    fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Self>;
}
