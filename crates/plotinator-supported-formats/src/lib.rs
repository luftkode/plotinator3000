use logs::SupportedLog;
use plotinator_log_if::{prelude::*, rawplot::path_data::GeoSpatialDataset};
use plotinator_mqtt_ui::serializable::SerializableMqttPlotData;
use plotinator_ui_file_io::{ParseUpdate, UpdateChannel};
use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    io::{self},
    path::{Path, PathBuf},
};

pub(crate) mod csv;
#[cfg(feature = "hdf5")]
#[cfg(not(target_arch = "wasm32"))]
mod hdf5;
pub(crate) mod logs;
pub(crate) mod parse_info;
pub use parse_info::ParseInfo;

use crate::csv::SupportedCsv;

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
    Csv(SupportedCsv),
    #[cfg(feature = "hdf5")]
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::upper_case_acronyms, reason = "The format is called HDF5")]
    HDF5(hdf5::SupportedHdf5Format),
    MqttData(SerializableMqttPlotData),
}

impl fmt::Display for SupportedFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Log(supported_log) => write!(f, "{supported_log}"),
            Self::Csv(supported_csv) => write!(f, "{supported_csv}"),
            Self::HDF5(supported_hdf5_format) => write!(f, "{supported_hdf5_format}"),
            Self::MqttData(serializable_mqtt_plot_data) => {
                write!(f, "{serializable_mqtt_plot_data}")
            }
        }
    }
}

fn mmap_file(file: &fs::File) -> io::Result<memmap2::Mmap> {
    #[allow(
        unsafe_code,
        reason = "If the user manages to drop a file and then delete that file before we are done parsing it then they deserve it"
    )]
    // SAFETY: It's safe as long as the underlying file is not modified before this function returns
    let mmap: memmap2::Mmap = unsafe { memmap2::Mmap::map(file)? };

    #[cfg(not(any(target_os = "windows", target_arch = "wasm32")))]
    mmap.advise(memmap2::Advice::Sequential)?;

    Ok(mmap)
}

impl SupportedFormat {
    /// Attempts to parse a log from raw content.
    pub fn parse_from_buf(content: &[u8], path: PathBuf, tx: UpdateChannel) -> io::Result<Self> {
        let log = SupportedLog::parse_from_buf(content, path, tx)?;
        Ok(Self::Log(log))
    }

    /// Parse a buffer of file contents known to be a csv file.
    ///
    /// Callers must check that it's a .csv file first
    pub fn parse_csv_from_buf(
        content: &[u8],
        path: PathBuf,
        tx: UpdateChannel,
    ) -> io::Result<Self> {
        let log = SupportedCsv::parse_from_buf(content, path, tx)?;
        Ok(Self::Csv(log))
    }

    /// Attempts to parse a log from a file path.
    ///
    /// This is how it is made available on native.
    pub fn parse_from_path(path: &Path, tx: UpdateChannel) -> anyhow::Result<Self> {
        plotinator_macros::profile_function!();

        tx.send(ParseUpdate::Started {
            path: path.to_path_buf(),
        });

        let file = fs::File::open(path)?;

        let log: Self = if plotinator_hdf5::path_has_hdf5_extension(path) {
            tx.send(ParseUpdate::Attempting {
                path: path.to_path_buf(),
                format_name: "hdf5".to_owned(),
            });
            Self::parse_hdf5_from_path(path, tx)?
        } else if let Some(ext) = path.extension()
            && (ext == "csv"
                || (ext == "txt"
                    && path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().contains("PPP"))))
        {
            tx.send(ParseUpdate::Attempting {
                path: path.to_path_buf(),
                format_name: "csv".to_owned(),
            });
            let mmap = mmap_file(&file)?;
            Self::parse_csv_from_buf(&mmap[..], path.to_path_buf(), tx)?
        } else {
            tx.send(ParseUpdate::Attempting {
                path: path.to_path_buf(),
                format_name: "regular file".to_owned(),
            });
            let mmap = mmap_file(&file)?;
            Self::parse_from_buf(&mmap[..], path.to_path_buf(), tx)?
        };

        log::debug!("Got: {}", log.descriptive_name());
        Ok(log)
    }

    #[cfg(feature = "hdf5")]
    #[cfg(not(target_arch = "wasm32"))]
    #[plotinator_proc_macros::log_time]
    fn parse_hdf5_from_path(path: &Path, tx: UpdateChannel) -> anyhow::Result<Self> {
        let h5file = hdf5::SupportedHdf5Format::from_path(path, tx)?;
        Ok(Self::HDF5(h5file))
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
            Self::Csv(c) => Some(c.parse_info()),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF5(_) => None,
            Self::MqttData(_) => None,
        }
    }

    /// Returns all the [`GeoSpatialData`] contained within, if any
    ///
    /// This is meant for after the regular plots have already been processed and the geo data can be sent
    /// to the map view
    pub fn geo_spatial_data(&self) -> Vec<GeoSpatialDataset> {
        let mut geo_spatial_data: Vec<GeoSpatialDataset> = vec![];
        match self {
            Self::Log(supported_log) => {
                for rp in supported_log.raw_plots() {
                    if let RawPlot::GeoSpatialDataset(geo_data) = rp {
                        geo_spatial_data.push(geo_data.clone());
                    }
                }
            }
            Self::Csv(supported_csv) => {
                for rp in supported_csv.raw_plots() {
                    if let RawPlot::GeoSpatialDataset(geo_data) = rp {
                        geo_spatial_data.push(geo_data.clone());
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF5(supported_hdf5_format) => {
                for rp in supported_hdf5_format.raw_plots() {
                    if let RawPlot::GeoSpatialDataset(geo_data) = rp {
                        geo_spatial_data.push(geo_data.clone());
                    }
                }
            }
            Self::MqttData(serializable_mqtt_plot_data) => {
                for rp in serializable_mqtt_plot_data.raw_plots() {
                    if let RawPlot::GeoSpatialDataset(geo_data) = rp {
                        geo_spatial_data.push(geo_data.clone());
                    }
                }
            }
        }
        geo_spatial_data
    }
}

impl Plotable for SupportedFormat {
    fn raw_plots(&self) -> &[RawPlot] {
        match self {
            Self::Log(l) => l.raw_plots(),
            Self::Csv(c) => c.raw_plots(),

            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF5(hdf) => hdf.raw_plots(),
            Self::MqttData(mqtt) => mqtt.raw_plots(),
        }
    }

    fn first_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            Self::Log(l) => l.first_timestamp(),
            Self::Csv(c) => c.first_timestamp(),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF5(hdf) => hdf.first_timestamp(),
            Self::MqttData(mqtt) => mqtt.first_timestamp(),
        }
    }

    fn descriptive_name(&self) -> &str {
        match self {
            Self::Log(l) => l.descriptive_name(),
            Self::Csv(c) => c.descriptive_name(),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF5(hdf) => hdf.descriptive_name(),
            Self::MqttData(mqtt) => mqtt.descriptive_name(),
        }
    }

    fn labels(&self) -> Option<&[PlotLabels]> {
        match self {
            Self::Log(l) => l.labels(),
            Self::Csv(c) => c.labels(),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF5(hdf) => hdf.labels(),
            Self::MqttData(mqtt) => mqtt.labels(),
        }
    }

    fn metadata(&self) -> Option<Vec<(String, String)>> {
        match self {
            Self::Log(l) => l.metadata(),
            Self::Csv(c) => c.metadata(),
            #[cfg(feature = "hdf5")]
            #[cfg(not(target_arch = "wasm32"))]
            Self::HDF5(hdf) => hdf.metadata(),
            Self::MqttData(mqtt) => mqtt.metadata(),
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
