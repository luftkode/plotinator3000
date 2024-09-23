use std::{fmt::Display, io};

pub mod util;

/// A given log should implement this trait
pub trait Log: Sized + Display {
    type Entry: LogEntry;
    /// Create a [Log] instance from a reader
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self>;
    /// Return a borrowed slice (list) of log entries
    fn entries(&self) -> &[Self::Entry];
}

/// A given log header should implement this
pub trait GitMetadata {
    fn project_version(&self) -> String;
    fn git_short_sha(&self) -> String;
    fn git_branch(&self) -> String;
    fn git_repo_status(&self) -> String;
}

/// A given log entry should implement this trait
pub trait LogEntry: Sized + Display {
    /// Create a [`LogEntry`] instance from a reader
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self>;
}
