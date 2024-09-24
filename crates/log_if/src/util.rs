use serde::{Deserialize, Serialize};

use crate::LogEntry;
use std::io;

/// Take a reader and parse [`LogEntry`]s from it until it returns an error,
/// then return a vector of all [`LogEntry`]s.
pub fn parse_to_vec<T: LogEntry, R: io::Read>(reader: &mut R) -> Vec<T> {
    let mut v = Vec::new();
    while let Ok(e) = T::from_reader(reader) {
        v.push(e);
    }
    v
}

/// Utility function for converting a slice of [`LogEntry`] to plot points by supplying extractor functions
/// detailing how to extract the timestamp (X) and the data (Y) from [`LogEntry`]s.
pub fn plot_points_from_log_entry<XF, YF, L: LogEntry>(
    log: &[L],
    x_extractor: XF,
    y_extractor: YF,
) -> Vec<[f64; 2]>
where
    XF: Fn(&L) -> f64,
    YF: Fn(&L) -> f64,
{
    log.iter()
        .map(|e| [x_extractor(e), y_extractor(e)])
        .collect()
}

/// Utility function for converting a slice of [`LogEntry`] to plot points by supplying a slice of normalized timestamps
/// and an extractor function detailing how to extract the data (Y) from [`LogEntry`]s.
pub fn plot_points_from_normalized_timestamp<F, L: LogEntry>(
    log: &[L],
    normalized_timestamps_ms: &[f64],
    y_extractor: F,
) -> Vec<[f64; 2]>
where
    F: Fn(&L) -> f64,
{
    log.iter()
        .zip(normalized_timestamps_ms)
        .map(|(e, ts)| [*ts, y_extractor(e)])
        .collect()
}

/// Where does the plot values typically fit within, e.g. RPM measurements will probably be in the thousands, while a duty cycle will be in percentage.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, Copy)]
pub enum ExpectedPlotRange {
    /// For plots where the value is 0.0-1.0 and corresponds to percentage 0-100%
    Percentage,
    OneToOneHundred,
    Thousands,
}
