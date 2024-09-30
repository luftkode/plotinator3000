use crate::prelude::*;
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
