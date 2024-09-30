use std::{fmt::Display, io};

use crate::plotable::Plotable;

/// A given log should implement this trait
pub trait Log: Plotable + Clone + Display + Send + Sync + Sized {
    type Entry: LogEntry;
    /// Create a [Log] instance from a reader
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self>;
    /// Return a borrowed slice (list) of log entries
    fn entries(&self) -> &[Self::Entry];
}

/// A given log entry should implement this trait
pub trait LogEntry: Sized + Display + Send + Sync {
    /// Create a [`LogEntry`] instance from a reader
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self>;
    /// Timestamp in nanoseconds
    fn timestamp_ns(&self) -> f64;
}

/// A given log header should implement this
///
/// If it does not, it returns [`None`] but it really should!
pub trait GitMetadata {
    fn project_version(&self) -> Option<String>;
    fn git_short_sha(&self) -> Option<String>;
    fn git_branch(&self) -> Option<String>;
    fn git_repo_status(&self) -> Option<String>;
}
