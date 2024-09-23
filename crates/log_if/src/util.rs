use std::io;

use crate::LogEntry;

/// Take a reader and parse [`LogEntry`]s from it until it returns an error,
/// then return a vector of all [`LogEntry`]s.
pub fn parse_to_vec<T: LogEntry, R: io::Read>(reader: &mut R) -> Vec<T> {
    let mut v = Vec::new();
    while let Ok(e) = T::from_reader(reader) {
        v.push(e);
    }
    v
}
