use logs::{SupportedLog, parse_info::ParseInfo};
use plotinator_log_if::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self},
    path::Path,
};

#[cfg(feature = "hdf5")]
#[cfg(not(target_arch = "wasm32"))]
mod hdf5;
pub(crate) mod logs;

/// Represents a supported format, which can be any of the supported format types.
///
/// This simply serves to encapsulate all the supported format in a single type
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(
    clippy::large_enum_variant,
    reason = "This enum is only created once when parsing an added file for the first time so optimizing memory for an instance of this enum is a waste of effort"
)]
pub enum SupportedFormat {
    Log(SupportedLog),
    #[cfg(feature = "hdf5")]
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::upper_case_acronyms, reason = "The format is called HDF...")]
    HDF(hdf5::SupportedHdf5Format),
}

impl SupportedFormat {
    /// Attempts to parse a log from raw content.
    ///
    /// This is how content is made available in a browser.
    pub(super) fn parse_from_buf(content: &[u8]) -> io::Result<Self> {
        let log = SupportedLog::parse_from_buf(content)?;
        Ok(Self::Log(log))
    }

    /// Attempts to parse a log from a file path.
    ///
    /// This is how it is made available on native.
    pub(crate) fn parse_from_path(path: &Path) -> io::Result<Self> {
        let file = fs::File::open(path)?;

        let log: Self = if plotinator_hdf5::path_has_hdf5_extension(path) {
            Self::parse_hdf5_from_path(path)?
        } else {
            #[allow(
                unsafe_code,
                reason = "If the user manages to drop a file and then delete that file before we are done parsing it then they deserve it"
            )]
            // SAFETY: It's safe as long as the underlying file is not modified before this function returns
            let mmap: memmap2::Mmap = unsafe { memmap2::Mmap::map(&file)? };
            Self::parse_from_buf(&mmap[..])?
        };
        log::debug!("Got: {}", log.descriptive_name());
        Ok(log)
    }

    #[cfg(feature = "hdf5")]
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_hdf5_from_path(path: &Path) -> io::Result<Self> {
        let h5file = hdf5::SupportedHdf5Format::from_path(path)?;
        Ok(Self::HDF(h5file))
    }

    #[cfg(not(feature = "hdf5"))]
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_hdf5_from_path(path: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Recognized '{}' as an HDF5 file. But the HDF5 feature is turned off.",
                path.display()
            ),
        ))
    }

    #[cfg(target_arch = "wasm32")]
    fn parse_hdf5_from_path(path: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Recognized '{}' as an HDF5 file. HDF5 files are only supported on the native version",
                path.display()
            ),
        ))
    }

    /// Returns [`None`] if there's no meaningful parsing information such as with HDF5 files.
    #[allow(
        clippy::unnecessary_wraps,
        reason = "HDF5 files are not supported on web and the lint is triggered when compiling for web since then only logs are supported which always have parse info"
    )]
    pub fn parse_info(&self) -> Option<ParseInfo> {
        match self {
            Self::Log(l) => Some(l.parse_info()),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF(_) => None,
        }
    }
}

impl Plotable for SupportedFormat {
    fn raw_plots(&self) -> &[RawPlot] {
        match self {
            Self::Log(l) => l.raw_plots(),

            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF(hdf) => hdf.raw_plots(),
        }
    }

    fn first_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            Self::Log(l) => l.first_timestamp(),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF(hdf) => hdf.first_timestamp(),
        }
    }

    fn descriptive_name(&self) -> &str {
        match self {
            Self::Log(l) => l.descriptive_name(),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF(hdf) => hdf.descriptive_name(),
        }
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        match self {
            Self::Log(l) => l.labels(),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF(hdf) => hdf.labels(),
        }
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        match self {
            Self::Log(l) => l.metadata(),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF(hdf) => hdf.metadata(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotinator_logs::mbed_motor_control::{pid::pidlog::PidLog, status::statuslog::StatusLog};
    use plotinator_test_util::*;

    #[test]
    fn test_supported_logs_dyn_vec() -> TestResult {
        let mut data = MBED_STATUS_V1_BYTES;
        let (status_log, _status_log_bytes_read) = StatusLog::from_reader(&mut data)?;

        let mut data = MBED_PID_V1_BYTES;
        let (pidlog, _pid_log_bytes_read) = PidLog::from_reader(&mut data)?;

        let v: Vec<Box<dyn Plotable>> = vec![Box::new(status_log), Box::new(pidlog)];
        assert_eq!(v.len(), 2);

        Ok(())
    }
}
